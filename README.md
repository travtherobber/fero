# fero
a lightweight terminal text editor

---

## index
* [installation](#installation)
* [technical features](#technical-features)
* [roadmap](#roadmap)
* [contact & support](#contact--support)

---

## installation
**TBD** *installation instructions are currently being finalized and will be available soon.*

---
 
## technical features
<details>
<summary><b>core architecture & state management</b></summary>

* **state-driven design**: utilizes a central `appstate` enum to handle transitions between the editor, file explorer, and configuration menus.
* **multitab buffer logic**: 
  * supports concurrent file buffers with independent cursor tracking.
  * indexed navigation for rapid switching between open files.
* **low-level terminal control**: built on `crossterm` for raw mode manipulation and 24-bit (true color) rendering.

</details>

<details>
<summary><b>editor & syntax engine</b></summary>

* **regex tokenization**: implements a custom syntax highlighter for:
  * **rust**: keywords, strings, and types.
  * **python**: built-ins and logic operators.
  * **bash**: shell commands and variables.
* **integrated explorer**:
  * recursive directory crawling with parent/child node logic.
  * real-time file tree expansion and direct buffer injection.
* **persistent configuration**:
  * uses `toml` serialization via `serde` and `toml-rs`.
  * automatic path discovery using the `dirs` crate for os-specific config storage.

</details>

<details>
<summary><b>live styling & personalization</b></summary>

* **palette engine**: users can modify hex codes (`#000000`) for the ui within a live-preview menu; changes are written back to `fero.toml` instantly.
* **rebindable keymap**:
  * internal command mapping system for custom workflows.
  * *note: most advanced shortcuts are disabled by default to prevent terminal multiplexer conflicts.*
* **cross-platform clipboard**: integrated via the `arboard` crate for seamless system-level copy/paste.

</details>

---

## roadmap
- [ ] 
- [ ] 
- [ ] 

---

## contact & support
* **email**: anguishedkitty@proton.me
* **github**: [github.com/travtherobber](https://github.com/travtherobber)
* **support**: $TravTheRobber (chime)