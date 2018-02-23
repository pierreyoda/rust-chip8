# rust-chip8 [![Build Status](https://travis-ci.org/pierreyoda/rust-chip8.svg?branch=master)](https://travis-ci.org/pierreyoda/rust-chip8)

A CHIP 8 emulator implemented in Rust and using SDL 2.
Created for learning purposes.

## Dependencies

- Rust : compiled against the latest [Rust *master* branch][rust-master]. The latest nightly installer should work.
- Cargo : Rust package manager.
- SDL2 : requires the development libraries and the associated [Rust binding][rust-sdl2].
- also uses the [time][rust-time], [rand][rust-rand], [log][rust-log] and [getopts][rust-getopts] crates

[rust-master]: https://github.com/rust-lang/rust
[rust-sdl2]: https://github.com/AngryLawyer/rust-sdl2
[rust-time]: https://github.com/rust-lang/time
[rust-rand]: https://github.com/rust-lang/rand
[rust-log]: https://github.com/rust-lang/log
[rust-getopts]: https://github.com/rust-lang/getopts

## Screenshots

Maze

![Maze](/img/maze.png?raw=true)

## Resources

The [Zophar] website offers a comprehensive collection of game and demo ROMs.

[Zophar]: https://www.zophar.net/pdroms/chip8.html
