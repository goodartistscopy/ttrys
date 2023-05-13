`ttrys` is what happens if you mix the `TTY` with `RuSt`.

![What it looks like (accelerated)](https://github.com/goodartistscopy/ttrys/assets/3617165/ffe9216d-8b34-4889-90c9-c4bc97f7402e "What it looks like (accelerated)")

This code is mainly used for playful exploration of the Rust language and other stuff. Don't expect a perfect, competitive caliber Tetris implementation. Working in a terminal imposes some limitations, most of the heavy lifting is done by the [crossterm](https://docs.rs/crossterm/latest/crossterm/) crate.

Usage
-----
```
$ cargo run --release
```

`[left]/[right]` move
`[up]/[down]` rotate
`[space]` hard-drop
`[P]` pause
`[Esc]` quit

Known limitations
-----------------
* Display glitches: sometimes an unwanted escape sequence is drawn; sreen tearing due to unoptimal cursor management
* Piece motions are basics (no "wall kick" motions)
* Not tested on Windows terminals

Things I might add
------------------
* Configurability: key bindings, color theme, stack size, gameplay options, etc.
* Better display method to limit artifacts
* Use the alternate terminal buffer when available
* Display a ghost piece to aid hard dropping; implement soft dropping
* Implement cascading gravity when clearing rows
* Add testing facilities to validate behavior
* Gameplay extras: random garbage penalties, score combos, wall traversing pieces, etc.
* Multi-player
