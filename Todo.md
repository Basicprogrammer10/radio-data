# Todo

## Modules

- [ ] Morse Code &mdash;
      [Reference](https://en.wikipedia.org/wiki/Morse_code) on Wikipedia.
      The duration of a dash is three times that of a dot.
      Each character is separated by a space of duration equal to that of a dot.
      Spaces are represented by a space of duration equal to three times that of a dot.
      Using command line args the user will be able to specify the duration of a dot, and the frequency of the tone.
  - [x] Module
  - [ ] Interface
  - [x] Data Transmission
  - [ ] Data Reception
- [ ] Binary Data Transmission &mdash; ill worry about this later
- [ ] Draw images with the spectrum analyzer

## Misc

- [ ] Generate other data types with TRNG module
- [ ] Add more TRNG tests ([source](https://www.random.org/analysis/Analysis2005.pdf))
- [x] Input and output gain control
- [x] Spectrum Analyzer audio pass-through
- [x] Support multiple channels in all modules
  - [x] Other modules
  - [x] DTMF Receiver
  - [x] Export to function
- [x] Allow using SmoothTones in the sequencer
- [x] Add documentation to all modules
- [x] Show RMS in the spectrum analyzer
- [x] Allow picking windowing functions for spectrum analyzer
- [x] Correctly implement windowing functions on the fft
- [x] Add a gain control to the spectrum analyzer
- [x] Move passthrough to its own module under audio
- [x] GUI Spectrum Analyzer
- [ ] Exit DTMF send when all data has been sent
- [ ] MORSE CODE DECODING
- [ ] Global system to play tone to actavate vox?