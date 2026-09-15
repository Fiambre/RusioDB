use rusqlite::{types::ValueRef, Connection};
use std::time::Duration;

pub const ROW_LIMIT: usize = 500;

#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected: usize,
    pub truncated: bool,
}

pub fn connect(path: &str) -> Result<Connection, String> {
    if path.trim().is_empty() {
        return Err("Ingresa una ruta SQLite o :memory:.".into());
    }
    let connection = Connection::open(path).map_err(|e| e.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(connection)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteCatalogEntry {
    pub name: String,
    pub is_view: bool,
}

pub fn catalog(connection: &Connection) -> Result<Vec<SqliteCatalogEntry>, String> {
    let mut statement = connection.prepare(
        "SELECT name, type FROM sqlite_schema WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name",
    ).map_err(|e| e.to_string())?;
    let entries = statement
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let kind: String = row.get(1)?;
            Ok(SqliteCatalogEntry {
                name,
                is_view: kind == "view",
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(entries)
}

pub fn preview_sql(table: &str) -> String {
    format!(
        "SELECT * FROM \"{}\" LIMIT {ROW_LIMIT};",
        table.replace('"', "\"\"")
    )
}

pub fn execute(connection: &Connection, sql: &str) -> Result<QueryResult, String> {
    if sql.trim().is_empty() {
        return Err("Escribe una sentencia SQL.".into());
    }
    let mut statement = connection.prepare(sql).map_err(|e| e.to_string())?;
    let columns = statement
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    if columns.is_empty() {
        let affected = statement.execute([]).map_err(|e| e.to_string())?;
        return Ok(QueryResult {
            affected,
            ..QueryResult::default()
        });
    }
    let mut cursor = statement.query([]).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    let mut truncated = false;
    while let Some(row) = cursor.next().map_err(|e| e.to_string())? {
        if rows.len() == ROW_LIMIT {
            truncated = true;
            // Finish stepping so statements with RETURNING complete correctly.
            continue;
        }
        let values = (0..columns.len())
            .map(|index| {
                row.get_ref(index)
                    .map(|value| match value {
                        ValueRef::Null => "NULL".into(),
                        ValueRef::Integer(n) => n.to_string(),
                        ValueRef::Real(n) => n.to_string(),
                        ValueRef::Text(bytes) => String::from_utf8_lossy(bytes).into_owned(),
                        ValueRef::Blob(bytes) => format!("[BLOB: {} bytes]", bytes.len()),
                    })
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        rows.push(values);
    }
    Ok(QueryResult {
        columns,
        rows,
        truncated,
        affected: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_changes_and_reads_types() {
        let db = connect(":memory:").unwrap();
        execute(&db, "CREATE TABLE sample (id INTEGER, value TEXT)").unwrap();
        assert_eq!(
            execute(&db, "INSERT INTO sample VALUES (1, NULL)")
                .unwrap()
                .affected,
            1
        );
        assert_eq!(
            catalog(&db).unwrap(),
            vec![SqliteCatalogEntry {
                name: "sample".into(),
                is_view: false,
            }]
        );
        assert_eq!(
            execute(&db, "SELECT * FROM sample").unwrap().rows[0],
            vec!["1", "NULL"]
        );
        assert!(execute(&db, "SELECT * FROM missing").is_err());
    }

    #[test]
    fn catalog_distinguishes_tables_from_views() {
        let db = connect(":memory:").unwrap();
        execute(&db, "CREATE TABLE t (id INTEGER)").unwrap();
        execute(&db, "CREATE VIEW v AS SELECT * FROM t").unwrap();
        let entries = catalog(&db).unwrap();
        assert_eq!(
            entries,
            vec![
                SqliteCatalogEntry {
                    name: "t".into(),
                    is_view: false
                },
                SqliteCatalogEntry {
                    name: "v".into(),
                    is_view: true
                },
            ]
        );
    }

    #[test]
    fn caps_results_and_quotes_identifiers() {
        let db = connect(":memory:").unwrap();
        execute(&db, "CREATE TABLE \"a\"\"b\" (id INTEGER)").unwrap();
        assert!(execute(&db, &preview_sql("a\"b")).is_ok());
        let result = execute(&db, "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<501) SELECT x FROM n").unwrap();
        assert_eq!(result.rows.len(), ROW_LIMIT);
        assert!(result.truncated);
    }
}
