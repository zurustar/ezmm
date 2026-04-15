# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

社内向けの動画バッチ編集ツール。役員メッセージ動画など、複数の動画に同じ編集テンプレートを適用して一括生成する。詳細は [docs/requirements.md](docs/requirements.md) を参照。

**重要**: 要件の変更・追加が生じた場合は、必ず `docs/requirements.md` を更新してから実装に入ること。

## Serena MCP Integration

This project includes a [Serena](https://github.com/oraios/serena) configuration at [.serena/project.yml](.serena/project.yml). Serena provides language-server-backed code intelligence tools (symbol search, rename, references, etc.) via MCP. When languages are configured in `.serena/project.yml`, prefer Serena's symbol-aware tools over plain text search for navigating and refactoring code.
