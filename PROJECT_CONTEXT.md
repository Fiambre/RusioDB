# Contexto del proyecto RusioDB

## Propósito

RusioDB es una aplicación de escritorio multiplataforma escrita en Rust para administrar bases de datos, tomando Navicat como referencia de producto y organización visual. La interfaz debe ser nativa en el sentido de estar implementada en Rust con un toolkit de escritorio, sin una aplicación web embebida.

El objetivo inmediato es construir un administrador usable para SQLite, PostgreSQL, MySQL y MongoDB. La primera entrega funcional prioriza conexiones, exploración del catálogo, consultas y resultados; las funciones avanzadas de Navicat se incorporarán por etapas.

## Decisiones técnicas

- Lenguaje: Rust, edición 2021.
- GUI: Iced 0.13.1, con `iced_aw` 0.12.2 para la barra de menús.
- Runtime asíncrono: Tokio.
- SQLite: rusqlite 0.32.1 con la característica `bundled`, por lo que SQLite se compila junto con la aplicación.
- PostgreSQL: crate `postgres` con `postgres-native-tls` y `native-tls`.
- MySQL: crate `mysql` 26 con `native-tls`.
- MongoDB: crate `mongodb` 3.x con la característica `sync` (API bloqueante `mongodb::sync::{Client, Database, Collection, Cursor}`), para encajar en el mismo patrón síncrono ejecutado vía `spawn_blocking` que usan los demás drivers, sin introducir un camino async nuevo. Mantiene los demás features por defecto del crate (`compat-3-0-0`→`bson-2`, `rustls-tls`, `dns-resolver`); no usar `default-features = false` sin volver a agregar esos defaults, porque el crate deja de compilar si se pierde la elección `bson-2`/`bson-3`.
- Autoupdate: `reqwest` (async, `default-features = false` + `json`/`native-tls`, consistente con el TLS directo que ya usan Postgres/MySQL) para consultar la API de Releases de GitHub y descargar el asset; `semver` (ya transitiva vía la cadena de iced, promovida a directa) para comparar versiones; `self-replace` para reemplazar el `.exe` en ejecución — no necesita ningún hook en `main.rs`: registra su lógica de "¿soy la copia auxiliar de autoeliminación?" en una sección de inicialización del CRT de Windows (`.CRT$XCV`) que corre antes de `main`, así que alcanza con tenerlo como dependencia.
- Ícono de Windows: `assets/icon.ico` se genera una vez desde `assets/cat.svg` con `examples/gen_icon.rs` (usa `resvg`/`ico` como `[dev-dependencies]`, no afectan el binario final) y se embebe en el `.exe` vía `build.rs` + `winresource` (sucesor mantenido de `winres`), gateado a `#[cfg(target_os = "windows")]`.
- Plataforma objetivo: Windows, macOS y Linux. El instalador y el autoupdater están implementados solo para Windows por ahora (ver "Riesgos y límites conocidos").
- Dependencias bloqueadas: `Cargo.lock` debe mantenerse actualizado y los comandos de CI/desarrollo deben usar `--locked`.

Iced dibuja sus controles y mantiene la interfaz en Rust. El código usa un modelo de estado y mensajes: los eventos de la UI producen `Message`, `App::update` modifica el estado o crea tareas, y `App::view` dibuja la vista actual.

## Estructura del código

- [`src/main.rs`](src/main.rs): arranque de Iced, tema, suscripción de teclado, tamaño inicial de ventana y carga de perfiles guardados (`App::load`).
- [`src/app.rs`](src/app.rs): estado de la aplicación, árbol de conexiones, menús, barra de herramientas, formulario de conexión, pestañas SQL, resultados, atajos y tareas asíncronas.
- [`src/drivers.rs`](src/drivers.rs): abstracción `Database`, configuración `Config`, enum `Driver`, catálogo `CatalogObject`/`ObjectKind` y adaptadores concretos para SQLite, PostgreSQL, MySQL y MongoDB.
- [`src/db.rs`](src/db.rs): operaciones específicas de SQLite, conversión de valores a texto, catálogo y límite de filas.
- [`src/connections.rs`](src/connections.rs): perfiles de conexión guardados (`ConnectionProfile`) y persistencia en JSON (`ConnectionsFile`, nunca incluye contraseñas), más el guardado opcional de contraseñas en el almacén de credenciales del sistema (`save_password`/`load_password`/`delete_password`).
- [`src/updater.rs`](src/updater.rs): chequeo (`check_for_update`), descarga (`download_update`) y aplicación (`apply_update`) de actualizaciones contra los Releases de GitHub. `parse_release_json` es la función pura (sin red) que decide si hay una versión más nueva y arma `UpdateInfo`.
- [`build.rs`](build.rs): embebe `assets/icon.ico` en el `.exe` de Windows vía `winresource`.
- [`examples/gen_icon.rs`](examples/gen_icon.rs): genera `assets/icon.ico` desde `assets/cat.svg`; uso único (`cargo run --example gen_icon`), no corre en cada build.
- [`installer/rusiodb.iss`](installer/rusiodb.iss): script de Inno Setup para el instalador de Windows (instalación por usuario, sin admin/UAC).
- [`.github/workflows/release.yml`](.github/workflows/release.yml): CI que compila, empaqueta el instalador y publica un Release de GitHub al pushear un tag `vX.Y.Z`.
- [`Cargo.toml`](Cargo.toml): dependencias y metadatos del paquete.
- [`README.md`](README.md): instrucciones de usuario, comandos y limitaciones conocidas.
- `target/debug/rusiodb.exe`: binario de desarrollo generado en Windows; no forma parte del código fuente.

## Modelo de drivers

`Driver` tiene cuatro variantes: `Sqlite`, `Postgres`, `Mysql` y `Mongo`. Cada conexión abierta se almacena como una variante de `Database` y se comparte con la UI mediante `Arc<Mutex<Database>>`.

La interfaz común expone:

- `Database::connect(&Config)`: valida la configuración y abre la conexión.
- `Database::catalog()`: obtiene el catálogo (tablas, vistas, vistas materializadas, funciones y procedimientos según el motor) para el árbol de conexiones.
- `Database::execute(sql)`: ejecuta SQL y devuelve columnas, filas, filas afectadas y si se alcanzó el límite visual.

Las consultas de catálogo son específicas de cada motor y devuelven objetos tipados (`ObjectKind::{Table, Collection, View, MaterializedView, Function, Procedure}`):

- PostgreSQL agrupa por esquema (`schema.nombre`) y agrega vistas materializadas (`pg_matviews`) y funciones/procedimientos (`pg_proc`/`pg_namespace`, distinguidos por `prokind`; requiere PostgreSQL 11+).
- MySQL agrupa tablas/vistas (`information_schema.tables`) y funciones/procedimientos (`information_schema.routines`), siempre acotado a `DATABASE()`; no se muestra un nivel de esquema separado porque coincide con la propia conexión.
- SQLite solo distingue tablas y vistas (`sqlite_schema.type`); no tiene funciones/procedimientos ni vistas materializadas catalogadas.
- MongoDB agrupa por base de datos (`listDatabases`, excluyendo `admin`/`local`/`config`) y lista colecciones y vistas de cada una (`listCollections`, excluyendo `system.*`); reutiliza el mismo nivel de agrupación por "esquema" del árbol que usa PostgreSQL. Un cliente Mongo ve varias bases a la vez, pero `Database::execute` solo puede consultar la base configurada en el formulario de conexión (segundo campo de `Database::Mongo(Client, String)`); consultar una colección de otra base visible en el árbol da un error claro, no falla en silencio.

Solo las tablas, colecciones, vistas y vistas materializadas son "previsualizables" (`ObjectKind::is_previewable`): generan un `SELECT * ... LIMIT 500` (o, para Mongo, `nombre_colección` + filtro `{}`) al hacer clic. Las funciones y procedimientos se muestran en el árbol pero no son clicables en esta iteración.

El formulario de conexión usa `:memory:` como valor inicial para SQLite. PostgreSQL usa el puerto 5432 por defecto, MySQL el 3306 y MongoDB el 27017. TLS está activado por defecto para conexiones remotas; la verificación del certificado contra el almacén del sistema es un toggle aparte ("Verificar certificado"), para poder cifrar sin rechazar certificados autofirmados de servidores internos. La contraseña se usa durante la conexión y se limpia del formulario inmediatamente después de intentarla; **nunca se persiste en `connections.json`**, ni siquiera junto al resto del perfil — si el usuario tilda "Guardar contraseña" (activado por defecto), se guarda aparte en el almacén de credenciales del sistema operativo.

Para MongoDB el editor no recibe SQL: la primera línea es el nombre de la colección y el resto (opcional) es un filtro en JSON — incluido JSON extendido de Mongo (`{"$oid": "..."}`) — parseado por `parse_mongo_filter` vía `serde_json::Value` → `bson::Document::try_from` (no `Document::from_reader`, que es para bytes BSON crudos, ni deserializar el texto directo a `bson::Document`, que no entiende Extended JSON). `documents_to_result` arma la misma forma `QueryResult` que los demás drivers: la unión de campos de nivel superior de todos los documentos se vuelve el set de columnas, y a cada fila le faltan como `"NULL"` los campos que ese documento no tiene; subdocumentos y arrays se muestran como su JSON compacto en una sola celda (sin aplanado recursivo). No hay soporte de `mongodb+srv://`/replica sets, aggregation pipelines ni inserts/updates/deletes en esta versión.

## Conexiones guardadas y persistencia

Los perfiles de conexión (nombre, driver, host/puerto/base de datos/usuario — nunca la contraseña) se guardan en un archivo JSON local (`ConnectionsFile` en `src/connections.rs`), en el directorio de configuración del sistema operativo (`dirs::config_dir()/RusioDB/connections.json`: `%APPDATA%` en Windows, `~/Library/Application Support` en macOS, `~/.config` en Linux). Se cargan al iniciar (`App::load`, usado desde `main.rs`) y se reescriben al crear, editar o eliminar un perfil.

Las contraseñas, cuando el usuario elige guardarlas, van al almacén de credenciales del sistema (Windows Credential Manager / macOS Keychain / Secret Service en Linux, vía el crate `keyring`) usando el id numérico de la conexión como clave — nunca al JSON. `Database::connect` recibe la contraseña ya resuelta (tipeada o recuperada del almacén) en `Config::password`; ese campo nunca se persiste.

`App::default()` (usado en los tests) nunca toca disco ni el almacén de credenciales real: `connections_path` queda en `None` (`persist_connections` se vuelve un no-op) y `keyring_enabled` queda en `false` (`sync_saved_password`/las llamadas a `connections::load_password`/`delete_password` se saltean), evitando que los tests escriban sobre el `connections.json` o el llavero del desarrollador. Solo `App::load()` (el arranque real, vía `main.rs`) habilita ambos.

## Conexiones simultáneas

RusioDB permite **varias conexiones abiertas a la vez**, cada una con su propio estado (`ConnState::{Disconnected, Connecting, Connected, Error}`). Cada pestaña de consulta queda atada a una conexión concreta (`Tab.connection_id`) y tiene su propio indicador de ejecución (`Tab.running`); ya no existe un `busy` global. Esto significa que:

- Ejecutar una consulta en una pestaña no bloquea otras pestañas atadas a conexiones distintas.
- Actualizar el catálogo de una conexión (`catalog_busy` por conexión) no afecta a las demás.
- Desconectar o eliminar una conexión no toca el estado de las otras.
- Solo el formulario de conexión, el árbol y algunas acciones puntuales (conectar/desconectar/eliminar) están gateadas por el estado específico de la conexión seleccionada, no por un flag global.

## Instalador y autoupdate (Windows)

RusioDB se distribuye como un instalador de Inno Setup (`installer/rusiodb.iss`, compilado por `.github/workflows/release.yml` al pushear un tag `vX.Y.Z`) que instala por usuario en `%LOCALAPPDATA%\Programs\RusioDB`, sin pedir admin/UAC. Esa elección es la que permite que el autoupdate funcione sin elevación: la app misma puede reemplazar su propio `.exe` porque el usuario ya es dueño del directorio de instalación.

El flujo de actualización vive en `src/updater.rs`, con estado en `App` (`update_available: Option<UpdateInfo>`, `updating: bool`) y un ciclo de `Message` que sigue la convención existente (variantes en participio pasado envolviendo `Result<T, String>`, ver `Connected`/`CatalogRefreshed`):

1. `App::load()` dispara `spawn_check_updates()` en su `Task::batch` inicial (chequeo silencioso al abrir); el ítem "Buscar actualizaciones" del menú Ayuda dispara lo mismo a demanda (`Message::CheckForUpdates`).
2. `updater::check_for_update()` consulta `GET /repos/Fiambre/RusioDB/releases/latest`, compara el `tag_name` (vía `semver`) contra `env!("CARGO_PKG_VERSION")` y arma un `UpdateInfo` si hay una versión más nueva con asset disponible para `x86_64-pc-windows-msvc`. Un 404 (todavía no hay releases publicados) se trata como "sin actualización", no como error. El parseo/comparación vive en la función pura `parse_release_json`, testeada sin red.
3. Si hay una versión nueva, aparece un banner (mismo patrón visual que el diálogo "Acerca de": no hay overlay/modal real en el proyecto, es un `container` más empujado al `Column` del layout principal) con un botón **Instalar**.
4. Al confirmar, `updater::download_update` baja el asset a un archivo temporal y `updater::apply_update` llama a `self_replace::self_replace` para reemplazar el `.exe` en ejecución — esta función no necesita ningún hook en `main.rs`, ya que `self-replace` registra su propia lógica de "¿soy la copia auxiliar?" en una sección de inicialización del CRT que corre antes de `main`.
5. Al terminar, la app relanza su propio ejecutable (`std::process::Command::new(current_exe).spawn()`) y sale con `iced::exit()`, igual que `Message::Exit`.

`App::default()` (tests) nunca dispara este chequeo: `update_available`/`updating` arrancan en `None`/`false` y ningún camino fuera de `App::load()`/una acción explícita del usuario los toca — mismo principio que `connections_path`/`keyring_enabled` para el JSON/llavero.

El ícono de la app se genera una vez con `examples/gen_icon.rs` (rasteriza `assets/cat.svg` a `assets/icon.ico` con `resvg`/`ico`, dependencias de desarrollo que no afectan el binario final) y se embebe en el `.exe` vía `build.rs` + `winresource`.

## Interfaz actual

La ventana contiene:

- Menús Archivo, Edición, Conexión, Consulta, Ver y Ayuda.
- Barra de herramientas para nueva conexión, nueva consulta, ejecutar y actualizar catálogo.
- Formulario de conexión (crear/editar/conectar) que cambia según el driver.
- Árbol lateral de conexiones guardadas, estilo Navicat: conexión → (esquema, PostgreSQL o base de datos, MongoDB) → carpetas Tablas/Colecciones/Vistas/Vistas materializadas/Funciones/Procedimientos → objetos.
- Pestañas independientes de consulta SQL, cada una atada a una conexión.
- Editor SQL con ejecución en hilo de trabajo.
- Grilla desplazable de resultados.
- Barra de estado con mensajes de progreso y errores.
- Banner de actualización disponible (Windows) cuando hay una versión nueva publicada.
- Tema claro y tema Tokyo Night.

Acciones y atajos actuales:

| Acción | Atajo |
| --- | --- |
| Nueva consulta | Ctrl/Cmd+N |
| Mostrar formulario de nueva conexión | Ctrl/Cmd+Shift+N |
| Ejecutar SQL | F5 o Ctrl/Cmd+Enter |
| Actualizar catálogo de la conexión seleccionada | F6 |
| Copiar SQL o resultados | Menú Edición o botones |

Un clic en una conexión del árbol la selecciona y expande/colapsa sus hijos; un doble clic conecta directo (SQLite siempre; PostgreSQL/MySQL también si tienen una contraseña guardada en el almacén de credenciales) o reabre el formulario pidiendo contraseña si no hay ninguna guardada. Eliminar una conexión pide confirmación con un segundo clic y también borra su contraseña guardada, si la tenía. Si una conexión falla al conectar, las demás conexiones abiertas no se ven afectadas.

## Comportamiento SQL

- La UI muestra como máximo 500 filas (`ROW_LIMIT`).
- SQLite prepara y ejecuta una sentencia; su comportamiento con scripts de varias sentencias no es uniforme con los demás drivers.
- PostgreSQL usa el protocolo de consulta simple y puede devolver mensajes de varias sentencias.
- MySQL usa `query_iter` y puede recorrer varios conjuntos de resultados.
- Para obtener resultados previsibles se debe ejecutar una sentencia por vez.
- Los valores se convierten a texto para la grilla. `NULL` se muestra como `NULL`; los BLOB se resumen con su tamaño.
- La copia de resultados produce texto separado por tabulaciones, no un CSV completo con escapes.
- No hay paginación, cancelación de consultas, edición de celdas ni exportación CSV.

Las operaciones de red y las consultas potencialmente bloqueantes se ejecutan con `tokio::task::spawn_blocking`, de modo que no bloqueen el hilo de la interfaz.

## Estado de verificación

En Windows están instalados Rust MSVC y Visual Studio Build Tools con C++.

Comandos verificados:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
```

Las pruebas locales cubren:

- Escrituras, lecturas, `NULL`, errores y transacciones SQLite.
- Distinción de tablas vs. vistas en el catálogo SQLite.
- Límite de 500 filas.
- Comillas correctas para identificadores PostgreSQL/MySQL.
- Validación de configuración sin exponer contraseñas.
- `ObjectKind::is_previewable` (qué tipos de objeto son clicables en el árbol).
- Persistencia de perfiles en JSON sin que aparezca nunca la contraseña.
- Que el `connections.json` guardado antes de agregar `tls_verify` siga cargando bien (default `true`).
- `App::default()` nunca toca disco ni el almacén de credenciales real (`connections_path` y `keyring_enabled`).
- Creación y conservación de pestañas; el driver del formulario es independiente de las pestañas ya abiertas.
- Editar un perfil distinto mientras otra conexión está conectando no se bloquea.
- Dos pestañas en dos conexiones distintas pueden quedar ejecutando (`running`) a la vez.
- Desconectar una conexión no afecta el estado de otra.
- Ctrl/Cmd+Enter dentro del editor.
- `parse_mongo_filter` (filtro vacío → documento vacío, JSON extendido de Mongo) y `documents_to_result` (unión de columnas entre documentos con campos distintos, faltantes como `NULL`) de MongoDB.
- `App::default()` nunca dispara el chequeo de actualizaciones (`update_available`/`updating`).
- `parse_release_json` del autoupdater: versión remota más nueva → `UpdateInfo`; igual o más vieja → `None`; asset faltante para esta plataforma o tag inválido → error.

Hay cuatro pruebas de integración ignoradas por defecto: PostgreSQL, MySQL, MongoDB y el chequeo de actualizaciones. Las tres primeras requieren variables `RUSIODB_PG_*`, `RUSIODB_MYSQL_*` o `RUSIODB_MONGO_*` con `HOST`, `PORT`, `DATABASE`, `USER`, `PASSWORD` y `TLS` (la de Mongo además necesita `RUSIODB_MONGO_COLLECTION`); la del autoupdater (`update_check_integration`) solo necesita salida a internet, pega contra la API real de GitHub.

## Cómo continuar el desarrollo

Antes de cambiar la arquitectura, conservar estas propiedades:

1. Las consultas y conexiones no deben bloquear la UI ni bloquearse entre sí (cada conexión y cada pestaña tienen su propio estado de "ocupado").
2. Las contraseñas no deben escribirse en archivos de texto plano ni logs, ni siquiera en `connections.json`; el único lugar donde pueden persistir es el almacén de credenciales del sistema operativo, y solo si el usuario lo pide.
3. La lógica común debe permanecer en la interfaz de `Database`; el SQL de catálogo y las diferencias de dialecto pertenecen a cada driver.
4. Los cambios de UI deben conservar el gateo por conexión/pestaña (no reintroducir un `busy` global), el refresco del catálogo y los resultados de la pestaña activa.
5. Las nuevas dependencias deben fijarse en `Cargo.lock` y comprobarse con `cargo ... --locked`.

Orden recomendado para próximos hitos:

1. Corregir y documentar diferencias de ejecución de scripts entre drivers.
2. Añadir cierre de pestañas con confirmación si hay cambios no ejecutados.
3. Añadir selector nativo de archivos para SQLite.
4. Implementar paginación, selección de celdas y exportación CSV segura.
5. Añadir metadatos de columnas, índices y claves al árbol (más allá de tablas/vistas/funciones/procedimientos).
6. ~~Permitir varias conexiones activas y asociar cada pestaña a una conexión.~~ Hecho: árbol de conexiones guardadas con persistencia JSON y estado por conexión/pestaña.
7. Pulido visual general (iconos, barra de herramientas tipo ribbon) más allá del árbol de conexiones.
8. Mostrar el código fuente al hacer clic en una función/procedimiento del árbol.
9. ~~Verificar empaquetado e interacción real en macOS y Linux.~~ Windows hecho (instalador Inno Setup + autoupdate). Pendiente: empaquetado e interacción real en macOS y Linux (el esquema de nombres de assets del release ya deja lugar para sumarlos).
10. ~~Agregar un cuarto motor (MongoDB).~~ Hecho en v1 acotada: conectar, árbol base de datos→colecciones/vistas, previsualizar/filtrar documentos con JSON. Pendiente para versiones futuras: `mongodb+srv://`/replica sets, aggregation pipelines, inserts/updates/deletes, selección de base por pestaña, aplanado recursivo de subdocumentos/arrays en la grilla.
11. ~~Instalador y autoupdate.~~ Hecho para Windows (`installer/rusiodb.iss`, `.github/workflows/release.yml`, `src/updater.rs`). Pendiente: firma de código (certificado pago), instaladores/autoupdate para macOS/Linux, preferencia para desactivar el chequeo automático al iniciar.

## Riesgos y límites conocidos

- Las conexiones PostgreSQL/MySQL no pueden validarse localmente sin servidores disponibles.
- TLS depende de las bibliotecas y certificados del sistema operativo.
- La grilla es una vista textual; aún no es un editor de datos estilo Navicat.
- Si el usuario destilda "Guardar contraseña" (o el almacén de credenciales del SO no está disponible), reconectar un perfil de PostgreSQL/MySQL vuelve a pedirla.
- El catálogo de PostgreSQL detecta la versión del servidor (`SHOW server_version_num`) para usar `pg_proc.prokind` (PG11+) solo cuando existe; en servidores más viejos todo `pg_proc` se clasifica como "Función" (correcto, ya que los procedimientos son un concepto de PG11+).
- Abortar una conexión mientras está en curso (`Connecting`) no está soportado; hay que esperar a que resuelva (éxito o error) antes de poder desconectarla.
- Las pruebas visuales y de interacción requieren ejecutar la aplicación en cada plataforma objetivo.
- MongoDB (v1): sin `mongodb+srv://`/replica sets, sin aggregation pipelines ni inserts/updates/deletes, una sola base de datos activa por conexión (la del formulario) aunque el árbol muestre todas las bases visibles, sin aplanado recursivo de subdocumentos/arrays en la grilla (se muestran como JSON compacto en la celda).
- El instalador y el `.exe` de Windows **no están firmados digitalmente** (no hay certificado de firma de código); Windows SmartScreen puede mostrar un aviso de "editor no reconocido" hasta que se firme o acumule reputación.
- El instalador/autoupdate solo están implementados para Windows; macOS y Linux siguen requiriendo compilar desde el código fuente.
- El chequeo de actualizaciones hace una llamada de red real a `api.github.com` en cada arranque de la app (además del disparo manual desde el menú Ayuda); no hay todavía una preferencia para desactivar el chequeo automático al iniciar.

## Principio de producto

RusioDB debe avanzar hacia una experiencia similar a Navicat sin copiar su implementación: navegación clara del árbol de bases de datos, consultas rápidas, resultados confiables y una separación estricta entre la UI y los drivers. Cada nuevo driver debe implementar conexión, catálogo, ejecución y conversión de resultados detrás de la misma interfaz para que el resto de la aplicación permanezca estable.
