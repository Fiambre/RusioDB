# RusioDB

Administrador de bases de datos de escritorio escrito en Rust con Iced. Interfaz compilada para Windows, macOS y Linux, con menús desplegables dentro de la ventana y drivers SQLite, PostgreSQL, MySQL y MongoDB.

## Ejecutar

```sh
cargo run --locked
```

Requiere Rust estable y un compilador C/C++. En Windows, Rust MSVC y Visual Studio Build Tools con C++. SQLite se compila junto a la aplicación. TLS usa las bibliotecas del sistema; Linux requiere también los paquetes de desarrollo de OpenSSL y las dependencias gráficas de Iced.

El ejecutable de desarrollo para Windows queda en `target/debug/rusiodb.exe`.

## Conexiones

Abre **Archivo → Nueva conexión** (o pulsa **Nueva conexión** en la barra de herramientas) para crear un perfil. El formulario cambia según el motor elegido:

| Motor | Campos | Árbol |
| --- | --- | --- |
| SQLite | Ruta de archivo o `:memory:` | Tablas y vistas |
| PostgreSQL | Servidor, puerto (5432), base de datos, usuario y contraseña | Tablas, vistas, vistas materializadas, funciones y procedimientos, agrupados por esquema |
| MySQL | Servidor, puerto (3306), base de datos, usuario y contraseña | Tablas, vistas, funciones y procedimientos de la base seleccionada |
| MongoDB | Servidor, puerto (27017), base de datos, usuario y contraseña | Colecciones y vistas de cada base visible, agrupadas por base de datos |

TLS está activado por defecto para los servidores y verifica el certificado mediante el almacén del sistema; ambas cosas se pueden desactivar por separado en el formulario ("TLS" y "Verificar certificado") — útil para servidores locales o con certificados autofirmados. **Las contraseñas nunca se escriben en `connections.json`**: si tildás "Guardar contraseña" (activado por defecto), se guardan en el almacén de credenciales del sistema operativo (Windows Credential Manager / macOS Keychain / Secret Service en Linux) y se recuperan solas al reconectar; si lo destildás, no queda nada guardado y hay que volver a tipearla cada vez.

Los perfiles (nombre, driver, host/puerto/base de datos/usuario) se guardan en un archivo JSON local y persisten entre sesiones. El botón **Guardar** del formulario los guarda sin conectar; **Conectar** los guarda (si son nuevos) y conecta de inmediato.

RusioDB soporta **varias conexiones abiertas a la vez**, cada una como un nodo independiente en el árbol lateral (estilo Navicat). Un clic en una conexión la selecciona y expande/colapsa sus hijos; un doble clic conecta (directo para SQLite; para PostgreSQL/MySQL reabre el formulario para pedir la contraseña, ya que nunca se guarda). Cada pestaña de consulta queda atada a la conexión desde la que se creó, y trabajar en una no bloquea las demás: podés ejecutar una consulta en una conexión mientras actualizás el catálogo o desconectás otra. Si una conexión falla al conectar, las demás conexiones abiertas no se ven afectadas. Eliminar una conexión guardada pide confirmación con un segundo clic en **Eliminar**.

## Menús y consultas

- **Archivo:** nueva conexión, nueva consulta y salir.
- **Edición:** copiar SQL (selección o texto completo), pegar, seleccionar todo y copiar resultados.
- **Conexión:** nueva conexión, editar/desconectar/eliminar la conexión seleccionada y actualizar su catálogo.
- **Consulta:** nueva pestaña y ejecutar SQL.
- **Ver:** mostrar/ocultar el árbol de conexiones y alternar tema claro/oscuro.
- **Ayuda:** información de la aplicación.

Atajos: **Ctrl+N** crea una consulta, **Ctrl+Shift+N** abre el formulario de nueva conexión, **F5** o **Ctrl+Enter** ejecuta la pestaña activa, **F6** actualiza el catálogo de la conexión seleccionada. En macOS se usa Cmd en lugar de Ctrl.

Cada consulta nueva abre una pestaña atada a la conexión seleccionada en el árbol. Pulsar una tabla/vista abre su consulta de vista previa en otra pestaña, atada a esa misma conexión. Ejecutar solo se habilita si la conexión de la pestaña activa está conectada; una pestaña en ejecución no bloquea otras pestañas de otras conexiones. Las consultas y las conexiones se procesan fuera del hilo de la interfaz.

Ejecuta una sentencia por vez, por ejemplo:

```sql
SELECT 42 AS respuesta, 'Hola desde RusioDB' AS mensaje;
```

La sesión se conserva entre ejecuciones, incluidas las transacciones explícitas. Si el catálogo no puede refrescarse después de una consulta correcta, sus resultados se conservan y se muestra el error del catálogo.

Para MongoDB el editor no recibe SQL: la primera línea es el nombre de la colección y el resto (opcional) es un filtro en JSON, incluido JSON extendido de Mongo (`{"$oid": "..."}`, `{"$gt": 5}`, etc.):

```
usuarios
{"activo": true}
```

Si se omite el filtro se usa `{}` (todos los documentos, hasta el límite de filas). Solo se pueden consultar colecciones de la base de datos configurada en la conexión, aunque el árbol muestre todas las bases visibles para el usuario. Subdocumentos y arrays se muestran como su JSON compacto en una sola celda, sin aplanado recursivo. No hay soporte de `mongodb+srv://`/replica sets, aggregation pipelines ni inserts/updates/deletes en esta versión.

## Verificación

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
```

Las pruebas locales cubren SQLite, transacciones, NULL, errores, límite de filas, identificadores por dialecto, configuración de conexiones, conservación de pestañas, persistencia de perfiles sin contraseñas, el comportamiento independiente entre conexiones/pestañas simultáneas y el parseo de filtros/documentos de MongoDB. Las tres pruebas de integración remota están ignoradas por defecto y requieren servidores configurados.

Para ejecutarlas, define variables de entorno con prefijo `RUSIODB_PG_`, `RUSIODB_MYSQL_` o `RUSIODB_MONGO_`: `HOST`, `PORT`, `DATABASE`, `USER`, `PASSWORD`, `TLS`. `PORT` toma el valor habitual si no se define; `TLS` solo se desactiva con el valor `false`. La prueba de Mongo además requiere `RUSIODB_MONGO_COLLECTION` con el nombre de una colección existente. Las pruebas ejecutan consultas de lectura y comprueban el catálogo y los errores.

```sh
cargo test postgres_integration -- --ignored
cargo test mysql_integration -- --ignored
cargo test mongodb_integration -- --ignored
```

La validación de conexiones reales PostgreSQL/MySQL/MongoDB, de interacción visual y de macOS/Linux requiere sus respectivos entornos.

## Límites actuales

La grilla muestra hasta 500 filas, sin paginación ni cancelación. SQLite y MySQL recorren las filas restantes sin agregarlas a la grilla; el driver PostgreSQL recibe el resultado completo antes de recortar la vista. Usa `LIMIT` en consultas grandes.

El soporte de scripts aún no es uniforme: SQLite ejecuta la primera sentencia; PostgreSQL y MySQL pueden ejecutar varias y se muestra el último resultado. Usa una sentencia por ejecución para un comportamiento consistente. Copiar resultados genera texto tabulado simple, sin escapes de CSV.

Las pestañas no se persisten al cerrar (los perfiles de conexión y, opcionalmente, sus contraseñas sí). Pendientes: guardar/abrir archivos SQL, cerrar pestañas con control de cambios, edición de celdas, exportación CSV, selector nativo de archivos y mostrar el código fuente de funciones/procedimientos.

## Estructura

- `src/app.rs`: estado, árbol de conexiones, menús, formularios, pestañas y tareas de interfaz.
- `src/drivers.rs`: interfaz común, configuración y adaptadores PostgreSQL/MySQL/SQLite/MongoDB.
- `src/db.rs`: ejecución y catálogo SQLite.
- `src/connections.rs`: perfiles de conexión guardados y persistencia en JSON; contraseñas opcionales en el almacén de credenciales del sistema (no en el JSON).
- `src/main.rs`: arranque de la aplicación y carga de perfiles guardados.
