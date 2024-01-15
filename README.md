# radio-data [![Rust](https://github.com/Basicprogrammer10/radio-data/actions/workflows/rust.yml/badge.svg)](https://github.com/Basicprogrammer10/radio-data/actions/workflows/rust.yml)

radio-data is a tool containing various command-line ham radio utilities.
They are used by connecting the audio input and output to your radios output and input, by now turing on voice activated transmission, you can fully automatically transfer data between multiple devices.
A basic ham radio with VOX was used over a full software defined radio just because they are more affordable.

## Features

### Spectrum Analyzer

I initially didn't have any plans of creating this module, but I thought might be useful for debugging, and it turned out to be one of the most intreating and fun parts of this project to work on.
It can either run in the command line using ANSI color codes or with a GUI (seen in the screenshot below).

```plain
Options:
  -f <fft-size>           The sample size of the FFT. Should be a power of 2. [default: 2048]
  -d <display-range>      The range of frequencies to display. In the format of `low..high`. [default: 15..14000]
  -w <window>             The window function to use on the samples [default: hann]
  -p                      Pass the audio through to the output device.
  -g <gain>               The gain to apply display, does not affect the passthrough. [default: 1.0]
  -t <display-type>       The method to use to display the spectrum. [possible values: console, window]
  -h, --help              Print help
```

![Spectrum Analyzer Screenshot](https://github.com/Basicprogrammer10/radio-data/assets/50306817/a8414a06-7da2-44cd-bb43-69e15e152c65)

### DTMF

The first actual information transfer method I added was using [DTMF](https://en.wikipedia.org/wiki/Dual-tone_multi-frequency_signaling) (dial) tones.
To use it, you will need to computers each hooked up to a radio, one running `radio-data dtmf receive` and another running `radio-data dtmf send <data>`.

After running the send command, the module will print out the dtmf code that was created from the input data, for "Hello World" you will get `A#2396A6A6D614#9D64#A636#D`.
Then it will play a tone for a second just to make sure none of the data is cut off by VOX and start playing the DTMF tones.
If all goes well, the receiver will print the bytes as they come in.
If both the start and end sequences are detected then the decoded message will be printed.

With just normal DTMF tones, there are 16 different symbols.
To encode data into DTMF, every nibble of every byte is converted to its respective tone.

<!-- ### Morse Code

### True Random

### Range -->

// TODO
