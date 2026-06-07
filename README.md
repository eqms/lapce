<h1 align="center">
  <a href="https://lapce.dev" target="_blank">
  <img src="extra/images/logo.png" width=200 height=200/><br>
  Lapce
  </a>
</h1>

<h4 align="center">Lightning-fast And Powerful Code Editor · Blitzschneller und leistungsstarker Code-Editor</h4>

<div align="center">
  <a href="https://github.com/lapce/lapce/actions/workflows/ci.yml" target="_blank">
    <img src="https://github.com/lapce/lapce/actions/workflows/ci.yml/badge.svg" />
  </a>
  <a href="https://discord.gg/n8tGJ6Rn6D" target="_blank">
    <img src="https://img.shields.io/discord/946858761413328946?logo=discord" />
  </a>
  <a href="https://docs.lapce.dev" target="_blank">
      <img src="https://img.shields.io/static/v1?label=Docs&message=docs.lapce.dev&color=blue" alt="Lapce Docs">
  </a>
</div>

<div align="center">

> **Language / Sprache**: [🇩🇪 Deutsch](#deutsche-dokumentation) | [🇬🇧 English](#english-documentation)

</div>

![](https://github.com/lapce/lapce/blob/master/extra/images/screenshot.png?raw=true)

---

## Deutsche Dokumentation

### Projektübersicht

Lapce (IPA: /læps/) ist vollständig in Rust geschrieben, mit einer Oberfläche auf Basis von [Floem](https://github.com/lapce/floem). Der Editor ist mit [Rope Science](https://xi-editor.io/docs/rope_science_00.html) aus dem [Xi-Editor](https://github.com/xi-editor/xi-editor) konzipiert, was blitzschnelle Berechnungen ermöglicht, und nutzt [wgpu](https://github.com/gfx-rs/wgpu) für das Rendering. Weitere Informationen zu den Funktionen findest du auf der [Hauptwebseite](https://lapce.dev), die Benutzerdokumentation auf [GitBook](https://docs.lapce.dev/).

- **Version**: 0.4.6
- **Rust-Edition**: 2024 (Minimum Rust-Version: 1.87.0)
- **Lizenz**: Apache License 2.0

### Hinweis: Härtungs-Fork

Dieses Repository ist ein **härtungsorientierter Fork** des Upstream-Lapce. Ziel ist es, die im Codebase-Audit identifizierten Engineering-Qualitätsthemen systematisch zu lösen: Laufzeit-Panics, fehlende Integritätsprüfung von Downloads, Performance-Engpässe und veraltete/unsichere Dependency-Pins.

**Kernwert:** Der Editor darf bei normalen Benutzeraktionen niemals abstürzen (panic), und jede heruntergeladene Binärdatei (Plugin, Self-Update, Remote-Proxy) muss vor der Ausführung integritätsgeprüft werden. Stabilität und Supply-Chain-Sicherheit haben Vorrang; alles andere ist sekundär. Verbesserungen können später Upstream angeboten werden, Mergebarkeit ist jedoch ein nachrangiges Ziel.

### Funktionen

- Integrierte LSP-Unterstützung ([Language Server Protocol](https://microsoft.github.io/language-server-protocol/)) für intelligente Code-Funktionen wie Autovervollständigung, Diagnosen und Code-Aktionen
- Modales Editieren als erstklassiges Konzept (Vim-ähnlich, umschaltbar)
- Integrierte Remote-Entwicklung, inspiriert von [VSCode Remote Development](https://code.visualstudio.com/docs/remote/remote-overview). Du erhältst das Gefühl einer „lokalen" Umgebung und gleichzeitig die volle Leistung eines Remote-Systems. Mit [Lapdev](https://lap.dev/) lassen sich Remote-Entwicklungsumgebungen verwalten.
- Plugins können in jeder Programmiersprache geschrieben werden, die in das [WASI](https://wasi.dev/)-Format kompiliert (C, Rust, [AssemblyScript](https://www.assemblyscript.org/))
- Integriertes Terminal, um Befehle im Arbeitsbereich auszuführen, ohne Lapce zu verlassen

### Installation

Vorgefertigte Releases für Windows, Linux und macOS findest du [hier](https://github.com/lapce/lapce/releases) oder über die [Installation mit einem Paketmanager](docs/installing-with-package-manager.md). Wenn du aus dem Quellcode kompilieren möchtest, gibt es eine [Anleitung](docs/building-from-source.md).

### Mitwirken

[Lapdev](https://lap.dev/), entwickelt vom Lapce-Team, ist ein Cloud-Entwicklungsumgebungs-Dienst ähnlich GitHub Codespaces. Über den Button am Ende dieses Dokuments gelangst du in eine vollständig eingerichtete Lapce-Entwicklungsumgebung, in der du den Code durchsuchen und sofort entwickeln kannst. Alle Abhängigkeiten sind vorinstalliert.

Richtlinien zum Mitwirken findest du in [`CONTRIBUTING.md`](CONTRIBUTING.md).

### Feedback & Kontakt

Der beliebteste Treffpunkt für Lapce-Entwickler und -Nutzer ist der [Discord-Server](https://discord.gg/n8tGJ6Rn6D).

Alternativ kannst du die Diskussion auf [Reddit](https://www.reddit.com/r/lapce/) verfolgen.

Es gibt außerdem einen [Matrix-Space](https://matrix.to/#/#lapce-editor:matrix.org), der mit den Inhalten des Discord-Servers verknüpft ist.

### Lizenz

Lapce wird unter der Apache License Version 2 veröffentlicht, einer Open-Source-Lizenz. Du darfst zum Projekt beitragen oder den Code frei verwenden, solange du die Bedingungen einhältst. Den Lizenztext findest du hier: [`LICENSE`](LICENSE).

---

## English Documentation

### Project Overview

Lapce (IPA: /læps/) is written in pure Rust, with a UI in [Floem](https://github.com/lapce/floem). It is designed with [Rope Science](https://xi-editor.io/docs/rope_science_00.html) from the [Xi-Editor](https://github.com/xi-editor/xi-editor), enabling lightning-fast computation, and leverages [wgpu](https://github.com/gfx-rs/wgpu) for rendering. More information about the features of Lapce can be found on the [main website](https://lapce.dev) and user documentation can be found on [GitBook](https://docs.lapce.dev/).

- **Version**: 0.4.6
- **Rust edition**: 2024 (minimum Rust version: 1.87.0)
- **License**: Apache License 2.0

### Note: Hardening Fork

This repository is a **hardening-focused fork** of upstream Lapce. It systematically resolves the engineering-quality concerns surfaced in the codebase audit: runtime panics, missing download integrity verification, performance bottlenecks, and outdated/unsafe dependency pins.

**Core value:** The editor must never panic on normal user actions, and every binary it downloads (plugin, self-update, remote proxy) must be integrity-verified before execution. Stability and supply-chain safety come first; everything else is secondary. Improvements may later be offered upstream, but mergeability is a secondary goal.

### Features

- Built-in LSP ([Language Server Protocol](https://microsoft.github.io/language-server-protocol/)) support to give you intelligent code features such as: completion, diagnostics and code actions
- Modal editing support as first class citizen (Vim-like, and toggleable)
- Built-in remote development support inspired by [VSCode Remote Development](https://code.visualstudio.com/docs/remote/remote-overview). Enjoy the benefits of a "local" experience, and seamlessly gain the full power of a remote system. We also have [Lapdev](https://lap.dev/) which can help manage your remote dev environments.
- Plugins can be written in programming languages that can compile to the [WASI](https://wasi.dev/) format (C, Rust, [AssemblyScript](https://www.assemblyscript.org/))
- Built-in terminal, so you can execute commands in your workspace, without leaving Lapce.

### Installation

You can find pre-built releases for Windows, Linux and macOS [here](https://github.com/lapce/lapce/releases), or [installing with a package manager](docs/installing-with-package-manager.md). If you'd like to compile from source, you can find the [guide](docs/building-from-source.md).

### Contributing

[Lapdev](https://lap.dev/), developed by the Lapce team, is a cloud dev env service similar to GitHub Codespaces. By clicking the button below, you'll be taken to a fully set up Lapce dev env where you can browse the code and start developing. All dependencies are pre-installed, so you can get straight to code.

Guidelines for contributing to Lapce can be found in [`CONTRIBUTING.md`](CONTRIBUTING.md).

### Feedback & Contact

The most popular place for Lapce developers and users is on the [Discord server](https://discord.gg/n8tGJ6Rn6D).

Or, join the discussion on [Reddit](https://www.reddit.com/r/lapce/) where we are just getting started.

There is also a [Matrix Space](https://matrix.to/#/#lapce-editor:matrix.org), which is linked to the content from the Discord server.

### License

Lapce is released under the Apache License Version 2, which is an open source license. You may contribute to this project, or use the code as you please as long as you adhere to its conditions. You can find a copy of the license text here: [`LICENSE`](LICENSE).

---

<div align="center">
  <a href="https://ws.lap.dev/#https://github.com/lapce/lapce" target="_blank">
        <img src="https://lap.dev/images/open-in-lapdev.svg?version=8" alt="Open in Lapdev">
  </a>
</div>
