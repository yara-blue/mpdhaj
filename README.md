# MPDhaj

A somewhat[^1] conformant reimplementation of the [Music Player Daemon](https://mpd.readthedocs.io/en/latest/user.html#introduction) that supports the needs of @dvdsk @p1n3appl3... and maybe the needs of our friends if they ask nicely or send patches.

![blahaj with headphones](https://i.redd.it/1bb5oe4xkwna1.jpg)

### TODO:

- [ ] actually make sound
  - [ ] linux
  - [ ] macos
  - [ ] windows (ew)
- [ ] watch music dir for changes
- [ ] fill out the rest of the tags
- [ ] follow symlinks in music dir
- [ ] queries
- [ ] package with nix
- [ ] home-manager service
- [ ] album art
- [ ] audio file format support
  - [ ] all the "normal" types in our libraries, hopefully rodio/symphonia has pure rust decoders for most of them...
  - [ ] fluidsynth/midi support
  - [ ] [game music emulator](https://github.com/libgme/game-music-emu) integration (@p1n3appl3 likes retro games and has some soundtracks like Super Mario World in her library...)
- [ ] [discord integration](https://github.com/jakestanger/mpd-discord-rpc)
- [ ] [scrobbling](https://github.com/FoxxMD/multi-scrobbler)
- [ ] see if it runs on android, try with [one](https://mafa.indi.software/) [of](https://gitlab.com/gateship-one/malp) [these](https://github.com/sludgefeast/MPDroid) clients

[^1]: We don't support domains or any of MPD's standard plugins, so really ours is just compatible with the core MPD protocol to let us use existing clients/players, not a drop-in replacement for MPD.

![another blahaj with headphones](https://i.imgur.com/2jGFQO1.jpeg)
