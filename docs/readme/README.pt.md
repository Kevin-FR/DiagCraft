# DiagCraft

[English](../../README.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [Español](README.es.md) · [Italiano](README.it.md) · **Português** · [简体中文](README.zh-CN.md)

**Conecte as suas ideias.** Um editor de diagramas nativo, local e de código aberto,
com uma interface em sete idiomas e uma API para agentes de IA.

[![CI](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml/badge.svg)](https://github.com/Kevin-FR/DiagCraft/actions/workflows/build.yml)
[![MIT](https://img.shields.io/badge/licença-MIT-blue.svg)](../../LICENSE)

![Interface do DiagCraft em inglês](../images/interface-en.png)

## Transferir

As aplicações e as respetivas somas de verificação SHA-256 estão disponíveis na
[página de versões](https://github.com/Kevin-FR/DiagCraft/releases).

| Plataforma | Pacote |
| --- | --- |
| macOS 13+, Apple Silicon | Aplicação `.app` num ZIP; arquivo CLI separado |
| Windows x86_64 | ZIP portátil com a aplicação e a CLI |
| Linux x86_64 | Pacote `.deb` para amd64 ou arquivo `.tar.gz`, com a aplicação e a CLI |

**A versão 0.6.0 é experimental** e acrescenta texto formatado dentro dos blocos. O pacote Mac tem assinatura ad hoc e não foi notarizado pela Apple.
Os pacotes Windows não estão assinados com um certificado de editor.
Consulte o [guia de instalação](../installation.md).

## Funcionalidades

- Texto formatado nos blocos: seleção de palavras, estilos, tamanhos e cores, com Aplicar/Cancelar.
- Interface em inglês, francês, alemão, espanhol, italiano, português e chinês simplificado.
- Verificação opcional de atualizações no GitHub, transferências verificadas por SHA-256 e instalação manual.
- Formas, textos, gráficos, oito bibliotecas de símbolos e nove modelos.
- Conectores ligados a formas ou com extremidades livres, pontos de ligação e cotovelos editáveis.
- Documentos com várias páginas, formatos de papel, tamanhos personalizados e ajuste ao desenho.
- Preenchimentos, gradientes, padrões, contornos, pontas de seta e sombras.
- Seleção múltipla, alinhamento, distribuição, guias, cancelamento de gestos e histórico de anulação.
- Navegação por trackpad, faixa de opções compacta por separador e painéis redimensionáveis com preferências guardadas.
- Paleta de comandos com ⌘K / Ctrl+K e ajuda de teclado pesquisável com F1.
- Ficheiros `.draw` editáveis, galeria local de projetos com versões e exportação SVG/PDF.
- Ajuste automático das formas ao texto, com medição dos tipos de letra e disposição partilhada entre a interface, SVG e PDF.
- Cópias locais automáticas para recuperação após uma falha, sem substituir os originais.
- CLI e servidor MCP locais: inspeção, validação e patches JSON atómicos.

O formato de documento **v7** e a API para agentes **v1** têm números de versão
próprios, independentes da aplicação. Os ficheiros dos formatos v1–v6 são migrados
em memória ao abrir. O ficheiro original permanece inalterado até guardar.

Não é necessária uma conta nem uma chave API para desenhar. O servidor MCP trabalha
numa pasta local escolhida pelo utilizador; a aplicação não contacta modelos de IA.

O idioma é escolhido em **Arquivo → Idioma e configurações…**. Os textos dos desenhos
permanecem inalterados. O pacote Mac contém apenas código `arm64` e utiliza as
bibliotecas de sistema do macOS: **não é necessário Rosetta**.

Os guias detalhados estão atualmente em francês:
[idiomas e atualizações](../langues-et-mises-a-jour.md), [comandos](../commandes.md),
[faixa de opções e painéis](../interface.md), [recuperação](../recuperation.md)
[ajuste do texto](../ajustement-texte.md) e [gestão de versões](../versioning.md).

[conteúdo dos blocos](../contenu-blocs.md) (FR).

## Desenvolvimento

Instale [Rust](https://rustup.rs/) e Python 3.11 ou posterior, e execute:

```sh
git clone https://github.com/Kevin-FR/DiagCraft.git
cd DiagCraft
cargo run --locked -p diagcraft
cargo run --locked -p diagcraft-cli -- --help
```

As dependências de sistema para compilar em Debian/Ubuntu estão indicadas no
[guia de desenvolvimento](../development.md).

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo deny check licenses sources advisories
python3 scripts/check-publication.py
python3 scripts/check-locales.py
python3 -m unittest discover -s packaging -p 'test_*.py'
```

## Estrutura do projeto

| Pasta | Função |
| --- | --- |
| `crates/core` | Modelo de documento, geometria, páginas, histórico, SVG/PDF, galeria e API para agentes |
| `apps/diagcraft` | Aplicação nativa egui/eframe |
| `apps/diagcraft-cli` | Executável de consola sem interface gráfica |
| `assets` | Tipos de letra, identidade visual e avisos de licença com proveniência e somas de verificação |
| `docs` | Arquitetura, formatos, desenvolvimento, segurança e versões |
| `packaging` | Criação de pacotes e recolha das licenças das dependências por plataforma |
| `examples` | Documentos de demonstração e ficheiros de teste de compatibilidade do formato |

A interface, a CLI e MCP partilham o mesmo motor, que não depende de egui.
Consulte a [arquitetura](../architecture.md) e as [escolhas de bibliotecas](../dependencies.md).

## Contribuir

As tarefas previstas e os critérios de aceitação estão em [ROADMAP.md](../../ROADMAP.md).
Grupos, rotação, importação de imagens, exportação PNG, conectores que evitam
obstáculos e instaladores integrados ainda estão por desenvolver. O texto SVG depende dos tipos de letra do visualizador.

Leia [CONTRIBUTING.md](../../CONTRIBUTING.md). Para trabalhar num desenho com um
agente, consulte o [contrato para agentes](../AGENTS-DRAW.md). Comunique vulnerabilidades
de acordo com [SECURITY.md](../../SECURITY.md). Estes documentos estão atualmente em francês.

## Licença e proveniência

O código do projeto é distribuído sob a [licença MIT](../../LICENSE). As dependências
e os tipos de letra mantêm as suas próprias licenças; os avisos são incluídos nos pacotes.
[THIRD_PARTY.md](../../THIRD_PARTY.md) apresenta as atribuições, e a
[auditoria de publicação](../licensing.md) explica os controlos e os seus limites.
