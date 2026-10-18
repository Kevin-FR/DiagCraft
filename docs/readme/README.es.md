# DiagCraft

[English](../../README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · **Español** · [Italiano](README.it.md) · [Português](README.pt.md) · [简体中文](README.zh-CN.md)

**Conecta tus ideas.** Un editor de diagramas nativo, local y de código abierto,
con una interfaz en siete idiomas y una API para agentes de IA.

[![CI](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml/badge.svg)](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml)
[![MIT](https://img.shields.io/badge/licencia-MIT-blue.svg)](../../LICENSE)

![Interfaz de DiagCraft en inglés](../images/interface-en.png)

## Descargar

Las aplicaciones y sus sumas de comprobación SHA-256 están en la
[página de versiones](https://github.com/Kevin-FR/DiagCraft/releases).

| Plataforma | Paquete |
| --- | --- |
| macOS 13+, Apple Silicon | Aplicación `.app` en un ZIP; archivo de CLI independiente |
| Windows x86_64 | ZIP portátil con la aplicación y la CLI |
| Linux x86_64 | Paquete `.deb` para amd64 o archivo `.tar.gz`, con la aplicación y la CLI |

**0.6.0 es experimental** y añade texto enriquecido dentro de los bloques.
El paquete para Mac tiene firma ad hoc y no ha sido notarizado por Apple.
Los paquetes de Windows no están firmados con un certificado de editor.
Consulta la [guía de instalación](../installation.md).

## Funciones

- Texto enriquecido en los bloques: selección de palabras, estilos, tamaños y colores, con Aplicar/Cancelar.
- Interfaz en inglés, francés, alemán, español, italiano, portugués y chino simplificado.
- Comprobaciones opcionales de actualizaciones en GitHub, descargas verificadas mediante SHA-256 e instalación manual.
- Formas, textos, gráficos, ocho bibliotecas de símbolos y nueve plantillas.
- Conectores unidos a formas o con extremos libres, puntos de conexión y codos editables.
- Documentos de varias páginas, formatos de papel, tamaños personalizados y ajuste al dibujo.
- Rellenos, degradados, patrones, contornos, puntas de flecha y sombras.
- Selección múltiple, alineación, distribución, guías, cancelación de gestos e historial para deshacer.
- Navegación con trackpad, cinta compacta por pestañas y paneles redimensionables con preferencias guardadas.
- Paleta de comandos con ⌘K / Ctrl+K y ayuda de teclado con búsqueda mediante F1.
- Archivos `.draw` editables, galería local de proyectos con versiones y exportación SVG/PDF.
- Ajuste automático de las formas al texto, con medición de fuentes y disposición compartida entre la interfaz, SVG y PDF.
- Copias locales automáticas para recuperarse de un fallo, sin sobrescribir los originales.
- CLI y servidor MCP locales: inspección, validación y parches JSON atómicos.

El formato de documento **v7** y la API para agentes **v1** tienen sus propios
números de versión, independientes de la aplicación. Los archivos de los formatos
v1–v6 se migran en memoria al abrirlos. El archivo original no cambia hasta que se guarda.

No necesitas una cuenta ni una clave API para dibujar. El servidor MCP trabaja
en una carpeta local elegida por ti; la aplicación no contacta con ningún modelo de IA.

El idioma se elige en **Archivo → Idioma y configuración…**. Los textos de tus
dibujos no cambian. El paquete para Mac solo contiene código `arm64` y utiliza
las bibliotecas del sistema macOS: **no se necesita Rosetta**.

Las guías detalladas están actualmente en francés:
[idiomas y actualizaciones](../langues-et-mises-a-jour.md), [comandos](../commandes.md),
[cinta y paneles](../interface.md), [recuperación](../recuperation.md)
[ajuste del texto](../ajustement-texte.md) y [versionado](../versioning.md).

[contenido de los bloques](../contenu-blocs.md) (FR).

## Desarrollo

Instala [Rust](https://rustup.rs/) y Python 3.11 o posterior, y ejecuta:

```sh
git clone https://github.com/Kevin-FR/DiagCraft.git
cd DiagCraft
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --help
```

Las dependencias del sistema para compilar en Debian/Ubuntu están en la
[guía de desarrollo](../development.md).

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
python3 scripts/check-locales.py
python3 -m unittest discover -s packaging -p 'test_*.py'
```

## Estructura del proyecto

| Directorio | Función |
| --- | --- |
| `crates/core` | Modelo de documento, geometría, páginas, historial, SVG/PDF, galería y API para agentes |
| `apps/diagcraft` | Aplicación nativa egui/eframe |
| `apps/diagcraft-cli` | Ejecutable de consola sin interfaz gráfica |
| `assets` | Fuentes, identidad visual y avisos con procedencia y sumas de comprobación |
| `docs` | Arquitectura, formatos, desarrollo, seguridad y versiones |
| `packaging` | Creación de paquetes y recopilación de licencias de dependencias por plataforma |
| `examples` | Documentos de demostración y archivos de prueba de compatibilidad del formato |

La interfaz, la CLI y MCP comparten el mismo motor, que no depende de egui.
Consulta la [arquitectura](../architecture.md) y la [elección de bibliotecas](../dependencies.md).

## Contribuir

Las tareas previstas y sus criterios de aceptación están en [ROADMAP.md](../../ROADMAP.md).
Los grupos, la rotación, la importación de imágenes, la exportación PNG, el trazado
de conectores que evitan obstáculos y los instaladores integrados están pendientes.
El texto en SVG depende de las fuentes del visor.

Lee [CONTRIBUTING.md](../../CONTRIBUTING.md). Para trabajar en un dibujo con un
agente, consulta el [contrato para agentes](../AGENTS-DRAW.md). Informa de las
vulnerabilidades siguiendo [SECURITY.md](../../SECURITY.md). Estos documentos están actualmente en francés.

## Licencia y procedencia

El código del proyecto se publica bajo la [licencia MIT](../../LICENSE).
Las dependencias y las fuentes conservan sus propias licencias; los avisos se
incluyen en los paquetes. [THIRD_PARTY.md](../../THIRD_PARTY.md) detalla las atribuciones,
y la [auditoría de publicación](../licensing.md) explica los controles y sus límites.
