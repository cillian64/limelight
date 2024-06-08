# Limelight

Driver hardware and software for the DMG1083-based LED tiles distributed by
[Limehouse Labs](https://led.limehouselabs.org/docs/tiles/dmg1083/).  Heavily
based on [eta's work](https://git.eta.st/eta/led-panel-zone).

The PCB is somewhere between eta's rev1 and rev2 designs - I use the pin
assignments from rev2 (so it'll work with the same software) but instead of a
discrete RP2040 I return to a Pi Pico footprint, because I plan to use a Pico
WH running standalone (so I want Wi-Fi) instead of USB-control.  And I use PTH
packages for the shift registers for ease of hand assembly.
