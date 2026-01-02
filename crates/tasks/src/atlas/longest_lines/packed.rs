//! Bit packe a longest line's distance and angle into a single `f32`.

/// The max and bitmask for a u22: 4,194,303.
const U22_MAX: u32 = (1 << 22) - 1;
/// The max and bitmask for a u10: 1023.
const U10_MAX: u32 = (1 << 10) - 1;

#[derive(Default, Debug, Clone, Copy)]
/// Line of sight data that can be packed within 32 bits.
pub struct LineOfSight(pub f32);

#[expect(
    clippy::big_endian_bytes,
    reason = "We don't care about the host's endedness, we're just transmuting"
)]
impl LineOfSight {
    /// Transmute to `u32` in order to do the bitshifting.
    const fn to_u32(self) -> u32 {
        u32::from_be_bytes(self.as_f32().to_be_bytes())
    }

    /// Distance in meters from the point of view to the visible point.
    pub const fn distance(self) -> u32 {
        (self.to_u32() >> 10u32) & U22_MAX
    }

    /// The angle of the line of sight from the point of view.
    pub fn angle(self) -> color_eyre::Result<u16> {
        Ok((self.to_u32() & U10_MAX).try_into()?)
    }

    /// The raw `f32` representation of the packed longest line. Is meaningless unless unpacked.
    pub const fn as_f32(self) -> f32 {
        self.0
    }
}
