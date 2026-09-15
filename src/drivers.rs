//! Motor-specific connections. The UI only sees this shared interface.
use crate::db::{self, QueryResult, ROW_LIMIT};
use mysql::prelude::Queryable;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Driver {
    Sqlite,
    Postgres,
    Mysql,
    Mongo,
}

impl Driver {
    pub const ALL: [Self; 4] = [Self::Sqlite, Self::Postgres, Self::Mysql, Self::Mongo];
    pub fn port(self) -> &'static str {
        match self {
            Self::Sqlite => "",
            Self::Postgres => "5432",
            Self::Mysql => "3306",
            Self::Mongo => "27017",
        }
    }
    pub fn sample(self) -> &'static str {
        match self {
            Self::Sqlite => "SELECT sqlite_version() AS version;",
            Self::Mongo => "// Escribí un nombre de colección y ejecutá para previsualizarla.",
            _ => "SELECT version() AS version;",
        }
    }
}
impl fmt::Display for Driver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sqlite => "SQLite",
            Self::Postgres => "PostgreSQL",
            Self::Mysql => "MySQL",
            Self::Mongo => "MongoDB",
        })
    }
}

#[derive(Clone)]
pub struct Config {
    pub name: String,
    pub driver: Driver,
    pub path: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub user: String,
    pub password: String,
    pub tls: bool,
    pub tls_verify: bool,
    /// Si guardar (o mantener guardada) la contraseña en el almacén de
    /// credenciales del sistema al conectar/guardar. Es puramente de la UI:
    /// no se persiste en `ConnectionProfile`, la fuente de verdad es si el
    /// almacén tiene o no una entrada para esa conexión.
    pub remember_password: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            name: String::new(),
            driver: Driver::Sqlite,
            path: ":memory:".into(),
            host: "localhost".into(),
            port: String::new(),
            database: String::new(),
            user: String::new(),
            password: String::new(),
            tls: true,
            tls_verify: true,
            remember_password: true,
        }
    }
}
impl Config {
    pub fn label(&self) -> String {
        if self.driver == Driver::Sqlite {
            format!("SQLite · {}", self.path)
        } else {
            format!(
                "{} · {}:{}/{}",
                self.driver, self.host, self.port, self.database
            )
        }
    }
    fn validate(&self) -> Result<u16, String> {
        if self.host.trim().is_empty()
            || self.database.trim().is_empty()
            || self.user.trim().is_empty()
        {
            return Err("Completa servidor, base de datos y usuario.".into());
        }
        self.port
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| "El puerto debe estar entre 1 y 65535.".into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Table,
    Collection,
    View,
    MaterializedView,
    Function,
    Procedure,
}
impl ObjectKind {
    pub fn folder_label(self) -> &'static str {
        match self {
            Self::Table => "Tablas",
            Self::Collection => "Colecciones",
            Self::View => "Vistas",
            Self::MaterializedView => "Vistas materializadas",
            Self::Function => "Funciones",
            Self::Procedure => "Procedimientos",
        }
    }
    pub fn is_previewable(self) -> bool {
        matches!(
            self,
            Self::Table | Self::Collection | Self::View | Self::MaterializedView
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogObject {
    pub schema: Option<String>,
    pub name: String,
    pub kind: ObjectKind,
}
impl CatalogObject {
    pub fn label(&self) -> String {
        match &self.schema {
            Some(schema) => format!("{schema}.{}", self.name),
            None => self.name.clone(),
        }
    }
    pub fn preview(&self, driver: Driver) -> String {
        if driver == Driver::Mongo {
            return format!("{}\n{{}}", self.name);
        }
        if driver == Driver::Sqlite && self.schema.is_none() {
            return db::preview_sql(&self.name);
        }
        let quote = |name: &str| {
            let delimiter = if driver == Driver::Mysql { '`' } else { '"' };
            format!(
                "{delimiter}{}{delimiter}",
                name.replace(delimiter, &format!("{delimiter}{delimiter}"))
            )
        };
        let qualified = match &self.schema {
            Some(schema) => format!("{}.{}", quote(schema), quote(&self.name)),
            None => quote(&self.name),
        };
        format!("SELECT * FROM {qualified} LIMIT {ROW_LIMIT};")
    }
}

pub enum Database {
    Sqlite(rusqlite::Connection),
    Postgres(Box<postgres::Client>),
    Mysql(Box<mysql::Conn>),
    /// El cliente Mongo ve varias bases a la vez; en esta iteración solo se
    /// consulta la base elegida en el formulario (segundo campo), aunque el
    /// árbol muestre todas las bases visibles.
    Mongo(Box<mongodb::sync::Client>, String),
}
impl fmt::Debug for Database {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Database(connected)")
    }
}
impl Database {
    pub fn connect(config: &Config) -> Result<Self, String> {
        if config.driver == Driver::Sqlite {
            return db::connect(&config.path).map(Self::Sqlite);
        }
        let port = config.validate()?;
        match config.driver {
            Driver::Postgres => {
                let mut options = postgres::Config::new();
                options
                    .host(&config.host)
                    .port(port)
                    .user(&config.user)
                    .password(&config.password)
                    .dbname(&config.database)
                    .connect_timeout(Duration::from_secs(10));
                let client = if config.tls {
                    options.ssl_mode(postgres::config::SslMode::Require);
                    let mut builder = native_tls::TlsConnector::builder();
                    if !config.tls_verify {
                        builder.danger_accept_invalid_certs(true);
                        builder.danger_accept_invalid_hostnames(true);
                    }
                    let tls = builder.build().map_err(|e| e.to_string())?;
                    options.connect(postgres_native_tls::MakeTlsConnector::new(tls))
                } else {
                    options.ssl_mode(postgres::config::SslMode::Disable);
                    options.connect(postgres::NoTls)
                }
                .map_err(postgres_error)?;
                Ok(Self::Postgres(Box::new(client)))
            }
            Driver::Mysql => {
                let options = mysql::OptsBuilder::new()
                    .ip_or_hostname(Some(config.host.clone()))
                    .tcp_port(port)
                    .user(Some(config.user.clone()))
                    .pass(Some(config.password.clone()))
                    .db_name(Some(config.database.clone()))
                    .tcp_connect_timeout(Some(Duration::from_secs(10)))
                    .ssl_opts(config.tls.then(|| {
                        let opts = mysql::SslOpts::default();
                        if config.tls_verify {
                            opts
                        } else {
                            opts.with_danger_accept_invalid_certs(true)
                                .with_danger_skip_domain_validation(true)
                        }
                    }));
                mysql::Conn::new(options)
                    .map(|conn| Self::Mysql(Box::new(conn)))
                    .map_err(|e| e.to_string())
            }
            Driver::Mongo => {
                let mut query = Vec::new();
                if config.tls {
                    query.push("tls=true".to_string());
                    if !config.tls_verify {
                        query.push("tlsAllowInvalidCertificates=true".to_string());
                        query.push("tlsAllowInvalidHostnames=true".to_string());
                    }
                }
                let query_string = if query.is_empty() {
                    String::new()
                } else {
                    format!("?{}", query.join("&"))
                };
                let uri = format!(
                    "mongodb://{}:{}@{}:{}/{}{query_string}",
                    percent_encode_userinfo(&config.user),
                    percent_encode_userinfo(&config.password),
                    config.host,
                    port,
                    config.database,
                );
                let client =
                    mongodb::sync::Client::with_uri_str(&uri).map_err(|e| e.to_string())?;
                Ok(Self::Mongo(Box::new(client), config.database.clone()))
            }
            Driver::Sqlite => unreachable!(),
        }
    }

    pub fn catalog(&mut self) -> Result<Vec<CatalogObject>, String> {
        match self {
            Self::Sqlite(connection) => Ok(db::catalog(connection)?
                .into_iter()
                .map(|entry| CatalogObject {
                    schema: None,
                    name: entry.name,
                    kind: if entry.is_view {
                        ObjectKind::View
                    } else {
                        ObjectKind::Table
                    },
                })
                .collect()),
            Self::Postgres(client) => {
                // `pg_proc.prokind` solo existe desde PostgreSQL 11 (antes no
                // había procedimientos, solo funciones); en servidores más
                // viejos esa columna directamente no existe, así que la
                // consulta se arma según la versión en vez de asumir prokind.
                let has_prokind = client
                    .query_one("SHOW server_version_num", &[])
                    .ok()
                    .and_then(|row| row.get::<_, String>(0).parse::<u32>().ok())
                    .is_some_and(|version| version >= 110000);
                let routine_kind = if has_prokind {
                    "CASE p.prokind WHEN 'p' THEN 'procedure' ELSE 'function' END"
                } else {
                    "'function'"
                };
                let routine_filter = if has_prokind {
                    "p.prokind IN ('f', 'p')"
                } else {
                    "true"
                };
                let query = format!(
                    "SELECT table_schema, table_name, 'table' FROM information_schema.tables \
                     WHERE table_type = 'BASE TABLE' AND table_schema NOT IN ('pg_catalog', 'information_schema') \
                     UNION ALL \
                     SELECT table_schema, table_name, 'view' FROM information_schema.tables \
                     WHERE table_type = 'VIEW' AND table_schema NOT IN ('pg_catalog', 'information_schema') \
                     UNION ALL \
                     SELECT schemaname, matviewname, 'materialized_view' FROM pg_matviews \
                     WHERE schemaname NOT IN ('pg_catalog', 'information_schema') \
                     UNION ALL \
                     SELECT n.nspname, p.proname, {routine_kind} \
                     FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace \
                     WHERE {routine_filter} AND n.nspname NOT IN ('pg_catalog', 'information_schema') \
                     ORDER BY 1, 3, 2"
                );
                let rows = client.query(&query, &[]).map_err(postgres_error)?;
                Ok(rows
                    .into_iter()
                    .filter_map(|row| {
                        let kind = match row.get::<_, &str>(2) {
                            "table" => ObjectKind::Table,
                            "view" => ObjectKind::View,
                            "materialized_view" => ObjectKind::MaterializedView,
                            "function" => ObjectKind::Function,
                            "procedure" => ObjectKind::Procedure,
                            _ => return None,
                        };
                        Some(CatalogObject {
                            schema: Some(row.get(0)),
                            name: row.get(1),
                            kind,
                        })
                    })
                    .collect())
            }
            Self::Mysql(client) => {
                let rows: Vec<(String, String, String)> = client
                    .query(
                        "SELECT TABLE_SCHEMA, TABLE_NAME, TABLE_TYPE FROM information_schema.tables \
                         WHERE TABLE_SCHEMA = DATABASE() \
                         UNION ALL \
                         SELECT ROUTINE_SCHEMA, ROUTINE_NAME, ROUTINE_TYPE FROM information_schema.routines \
                         WHERE ROUTINE_SCHEMA = DATABASE() \
                         ORDER BY 3, 2",
                    )
                    .map_err(|e| e.to_string())?;
                Ok(rows
                    .into_iter()
                    .filter_map(|(schema, name, kind)| {
                        let kind = match kind.as_str() {
                            "BASE TABLE" => ObjectKind::Table,
                            "VIEW" => ObjectKind::View,
                            "FUNCTION" => ObjectKind::Function,
                            "PROCEDURE" => ObjectKind::Procedure,
                            _ => return None,
                        };
                        Some(CatalogObject {
                            schema: Some(schema),
                            name,
                            kind,
                        })
                    })
                    .collect())
            }
            Self::Mongo(client, _active_db) => {
                let mut objects = Vec::new();
                let names = client
                    .list_database_names()
                    .run()
                    .map_err(|e| e.to_string())?;
                for db_name in names
                    .into_iter()
                    .filter(|n| !matches!(n.as_str(), "admin" | "local" | "config"))
                {
                    let db = client.database(&db_name);
                    let specs = db.list_collections().run().map_err(|e| e.to_string())?;
                    for spec in specs {
                        let spec = spec.map_err(|e| e.to_string())?;
                        if spec.name.starts_with("system.") {
                            continue;
                        }
                        let kind = if spec.collection_type == mongodb::results::CollectionType::View
                        {
                            ObjectKind::View
                        } else {
                            ObjectKind::Collection
                        };
                        objects.push(CatalogObject {
                            schema: Some(db_name.clone()),
                            name: spec.name,
                            kind,
                        });
                    }
                }
                Ok(objects)
            }
        }
    }

    pub fn execute(&mut self, sql: &str) -> Result<QueryResult, String> {
        if sql.trim().is_empty() {
            return Err("Escribe una consulta SQL.".into());
        }
        match self {
            Self::Sqlite(connection) => db::execute(connection, sql),
            Self::Postgres(client) => {
                let mut result = QueryResult::default();
                let mut completed = false;
                for message in client.simple_query(sql).map_err(postgres_error)? {
                    match message {
                        postgres::SimpleQueryMessage::RowDescription(columns) => {
                            completed = false;
                            result = QueryResult {
                                columns: columns.iter().map(|c| c.name().to_owned()).collect(),
                                ..QueryResult::default()
                            };
                        }
                        postgres::SimpleQueryMessage::Row(row) => {
                            if result.rows.len() < ROW_LIMIT {
                                result.rows.push(
                                    (0..row.len())
                                        .map(|i| row.get(i).unwrap_or("NULL").to_owned())
                                        .collect(),
                                );
                            } else {
                                result.truncated = true;
                            }
                        }
                        postgres::SimpleQueryMessage::CommandComplete(count) => {
                            if completed {
                                result = QueryResult::default();
                            }
                            result.affected = count as usize;
                            completed = true;
                        }
                        _ => {}
                    }
                }
                Ok(result)
            }
            Self::Mysql(client) => {
                let mut query = client.query_iter(sql).map_err(|e| e.to_string())?;
                let mut result = QueryResult::default();
                while let Some(mut set) = query.iter() {
                    result = QueryResult {
                        columns: set
                            .columns()
                            .as_ref()
                            .iter()
                            .map(|c| c.name_str().into_owned())
                            .collect(),
                        affected: set.affected_rows() as usize,
                        ..QueryResult::default()
                    };
                    for row in &mut set {
                        let row = row.map_err(|e| e.to_string())?;
                        if result.rows.len() < ROW_LIMIT {
                            result
                                .rows
                                .push(row.unwrap().into_iter().map(mysql_value).collect());
                        } else {
                            result.truncated = true;
                        }
                    }
                }
                Ok(result)
            }
            Self::Mongo(client, active_db) => {
                let mut lines = sql.splitn(2, '\n');
                let collection_name = lines.next().unwrap_or("").trim();
                if collection_name.is_empty() {
                    return Err("Escribe el nombre de una colección en la primera línea.".into());
                }
                let filter_text = lines.next().unwrap_or("").trim();
                let filter = parse_mongo_filter(filter_text)?;
                let collection = client
                    .database(active_db)
                    .collection::<mongodb::bson::Document>(collection_name);
                let cursor = collection
                    .find(filter)
                    .limit(ROW_LIMIT as i64)
                    .run()
                    .map_err(|e| e.to_string())?;
                let docs = cursor
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                let truncated = docs.len() == ROW_LIMIT;
                Ok(documents_to_result(docs, truncated))
            }
        }
    }
}

/// Convierte texto ingresado por el usuario a un filtro Mongo, entendiendo el
/// Extended JSON de Mongo (`{"$oid": "..."}`, `{"$date": "..."}`, etc.) — por
/// eso pasa por `serde_json::Value` y `bson::Document::try_from` en vez de
/// deserializar el texto directo a `bson::Document` (eso no entendería esa
/// sintaxis) o de pasar por `Document::from_reader` (que es para bytes BSON
/// crudos, no JSON de texto).
fn parse_mongo_filter(text: &str) -> Result<mongodb::bson::Document, String> {
    if text.is_empty() {
        return Ok(mongodb::bson::Document::new());
    }
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let map = value
        .as_object()
        .ok_or("El filtro debe ser un objeto JSON, por ejemplo {}")?
        .clone();
    mongodb::bson::Document::try_from(map).map_err(|e| e.to_string())
}

/// Arma una `QueryResult` (la misma forma que usan los otros tres motores)
/// a partir de documentos Mongo: la unión de los campos de nivel superior de
/// todos los documentos se vuelve el set de columnas, y a cada fila le
/// faltan como "NULL" los campos que ese documento no tiene. Sub-documentos y
/// arrays se muestran como su JSON compacto en una sola celda — sin aplanado
/// recursivo en esta iteración.
fn documents_to_result(docs: Vec<mongodb::bson::Document>, truncated: bool) -> QueryResult {
    let mut columns: Vec<String> = Vec::new();
    for doc in &docs {
        for key in doc.keys() {
            if !columns.contains(key) {
                columns.push(key.clone());
            }
        }
    }
    let rows = docs
        .iter()
        .map(|doc| {
            columns
                .iter()
                .map(|col| match doc.get(col) {
                    None | Some(mongodb::bson::Bson::Null) => "NULL".to_string(),
                    Some(value) => bson_cell(value),
                })
                .collect()
        })
        .collect();
    QueryResult {
        columns,
        rows,
        affected: 0,
        truncated,
    }
}

fn bson_cell(value: &mongodb::bson::Bson) -> String {
    match value {
        mongodb::bson::Bson::String(s) => s.clone(),
        mongodb::bson::Bson::Boolean(b) => b.to_string(),
        mongodb::bson::Bson::Int32(n) => n.to_string(),
        mongodb::bson::Bson::Int64(n) => n.to_string(),
        mongodb::bson::Bson::Double(n) => n.to_string(),
        mongodb::bson::Bson::ObjectId(id) => id.to_hex(),
        other => other.clone().into_relaxed_extjson().to_string(),
    }
}

fn percent_encode_userinfo(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn postgres_error(error: postgres::Error) -> String {
    match error.as_db_error() {
        Some(detail) => format!("{} (SQLSTATE {})", detail.message(), detail.code().code()),
        None => error.to_string(),
    }
}

fn mysql_value(value: mysql::Value) -> String {
    match value {
        mysql::Value::NULL => "NULL".into(),
        mysql::Value::Bytes(bytes) => match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => format!("[BLOB: {} bytes]", error.as_bytes().len()),
        },
        other => other.as_sql(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_each_identifier_for_each_driver() {
        let table = CatalogObject {
            schema: Some("a\"b".into()),
            name: "x`y".into(),
            kind: ObjectKind::Table,
        };
        assert_eq!(
            table.preview(Driver::Postgres),
            "SELECT * FROM \"a\"\"b\".\"x`y\" LIMIT 500;"
        );
        assert_eq!(
            table.preview(Driver::Mysql),
            "SELECT * FROM `a\"b`.`x``y` LIMIT 500;"
        );
    }
    #[test]
    fn object_kind_previewability_matches_leaf_interactivity() {
        assert!(ObjectKind::Table.is_previewable());
        assert!(ObjectKind::Collection.is_previewable());
        assert!(ObjectKind::View.is_previewable());
        assert!(ObjectKind::MaterializedView.is_previewable());
        assert!(!ObjectKind::Function.is_previewable());
        assert!(!ObjectKind::Procedure.is_previewable());
    }
    #[test]
    fn mongo_preview_is_a_collection_name_with_an_empty_filter() {
        let collection = CatalogObject {
            schema: Some("mydb".into()),
            name: "users".into(),
            kind: ObjectKind::Collection,
        };
        assert_eq!(collection.preview(Driver::Mongo), "users\n{}");
    }
    #[test]
    fn parse_mongo_filter_defaults_to_empty_document_and_parses_extended_json() {
        assert_eq!(parse_mongo_filter("").unwrap(), mongodb::bson::doc! {});
        assert_eq!(
            parse_mongo_filter("{\"age\": {\"$gt\": 5}}").unwrap(),
            mongodb::bson::doc! { "age": { "$gt": 5 } }
        );
        assert!(parse_mongo_filter("not json").is_err());
        assert!(parse_mongo_filter("[1, 2, 3]").is_err());
    }
    #[test]
    fn documents_to_result_unions_columns_and_fills_missing_fields_with_null() {
        let docs = vec![
            mongodb::bson::doc! { "name": "Ana", "age": 30 },
            mongodb::bson::doc! { "name": "Beto", "active": true },
        ];
        let result = documents_to_result(docs, false);
        assert_eq!(result.columns, vec!["name", "age", "active"]);
        assert_eq!(
            result.rows,
            vec![
                vec!["Ana".to_string(), "30".to_string(), "NULL".to_string()],
                vec!["Beto".to_string(), "NULL".to_string(), "true".to_string()],
            ]
        );
        assert!(!result.truncated);
    }
    #[test]
    fn shared_interface_keeps_sqlite_session_and_transactions() {
        let mut connection = Database::connect(&Config::default()).unwrap();
        connection
            .execute("CREATE TABLE sample (id INTEGER)")
            .unwrap();
        connection.execute("BEGIN").unwrap();
        connection.execute("INSERT INTO sample VALUES (1)").unwrap();
        connection.execute("ROLLBACK").unwrap();
        assert!(connection
            .execute("SELECT * FROM sample")
            .unwrap()
            .rows
            .is_empty());
        let catalog = connection.catalog().unwrap();
        assert_eq!(catalog[0].name, "sample");
        assert_eq!(catalog[0].kind, ObjectKind::Table);
    }
    #[test]
    fn validates_remote_config_without_exposing_password() {
        let config = Config {
            driver: Driver::Postgres,
            password: "secret".into(),
            ..Config::default()
        };
        assert!(config.validate().is_err());
        assert!(!config.label().contains("secret"));
    }

    fn remote_smoke(driver: Driver, prefix: &str) {
        let config = Config {
            driver,
            host: std::env::var(format!("{prefix}_HOST")).expect("Falta HOST de pruebas"),
            port: std::env::var(format!("{prefix}_PORT")).unwrap_or_else(|_| driver.port().into()),
            database: std::env::var(format!("{prefix}_DATABASE"))
                .expect("Falta DATABASE de pruebas"),
            user: std::env::var(format!("{prefix}_USER")).expect("Falta USER de pruebas"),
            password: std::env::var(format!("{prefix}_PASSWORD")).unwrap_or_default(),
            tls: std::env::var(format!("{prefix}_TLS"))
                .map(|value| value != "false")
                .unwrap_or(true),
            ..Config::default()
        };
        let mut database =
            Database::connect(&config).expect("No se pudo conectar al servidor de pruebas");
        let result = database
            .execute("SELECT 42 AS answer, NULL AS missing, 'texto' AS sample")
            .unwrap();
        assert_eq!(result.columns, vec!["answer", "missing", "sample"]);
        assert_eq!(result.rows, vec![vec!["42", "NULL", "texto"]]);
        let empty = database
            .execute("SELECT 1 AS empty_result WHERE 1 = 0")
            .unwrap();
        assert_eq!(empty.columns, vec!["empty_result"]);
        assert!(empty.rows.is_empty());
        assert!(database.catalog().is_ok());
        assert!(database
            .execute("SELECT * FROM rusiodb_nonexistent_test_table_9f271")
            .is_err());
    }

    #[test]
    #[ignore = "Requiere un servidor PostgreSQL y variables RUSIODB_PG_*"]
    fn postgres_integration() {
        remote_smoke(Driver::Postgres, "RUSIODB_PG");
    }

    #[test]
    #[ignore = "Requiere un servidor MySQL y variables RUSIODB_MYSQL_*"]
    fn mysql_integration() {
        remote_smoke(Driver::Mysql, "RUSIODB_MYSQL");
    }

    #[test]
    #[ignore = "Requiere un servidor MongoDB, variables RUSIODB_MONGO_* y una colección conocida"]
    fn mongodb_integration() {
        let config = Config {
            driver: Driver::Mongo,
            host: std::env::var("RUSIODB_MONGO_HOST").expect("Falta HOST de pruebas"),
            port: std::env::var("RUSIODB_MONGO_PORT")
                .unwrap_or_else(|_| Driver::Mongo.port().into()),
            database: std::env::var("RUSIODB_MONGO_DATABASE").expect("Falta DATABASE de pruebas"),
            user: std::env::var("RUSIODB_MONGO_USER").expect("Falta USER de pruebas"),
            password: std::env::var("RUSIODB_MONGO_PASSWORD").unwrap_or_default(),
            tls: std::env::var("RUSIODB_MONGO_TLS")
                .map(|value| value != "false")
                .unwrap_or(true),
            ..Config::default()
        };
        let collection =
            std::env::var("RUSIODB_MONGO_COLLECTION").expect("Falta COLLECTION de pruebas");
        let mut database =
            Database::connect(&config).expect("No se pudo conectar al servidor de pruebas");
        let catalog = database.catalog().unwrap();
        assert!(catalog
            .iter()
            .any(|object| object.schema.as_deref() == Some(config.database.as_str())));
        let result = database.execute(&format!("{collection}\n{{}}")).unwrap();
        assert!(result.rows.len() <= ROW_LIMIT);
        assert!(database
            .execute("rusiodb_nonexistent_test_collection_9f271\n{ invalid json")
            .is_err());
    }
}
