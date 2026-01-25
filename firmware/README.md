`cargo build` should work to build an rp235x elf at
target/thumbv8m.main-none-eabihf/debug/limelight.  The elf2uf2-rs in brew
doesn't seem to support rp235x so instead use picotool:

cp target/thumbv8m.main-none-eabihf/debug/limelight limelight.elf
picotool uf2 convert limelight.elf limelight.uf2

Then just copy the uf2 file to your Pico 2.

Debug builds work, but release builds have vastly better frame-rate.
