# Original synthetic fixtures

`jpeg-display.jpg` is the generated gradient from
`scripts/verify-jpeg-fidelity.py`, cleaned with orientation preservation enabled.
It contains orientation 6, EXIF print density 300 × 150 pixels per inch and an
sRGB ICC profile. It contains no author metadata. The desktop test verifies
that these retained fields do not enable cleanup until orientation preservation
is disabled; the independent Python suite verifies the actual image values.
