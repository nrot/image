//!  Utilities

use num_iter::range_step;
use std::iter::repeat;

#[cfg(feature = "async")]
pub enum ReaderUnion{
    Reader(std::io::Read),
    AsyncRead(tokio::io::AsyncRead),
}

#[cfg(not(feature = "async"))]
pub enum ReaderUnion<R: std::io::Read> {
    Reader(std::io::Read),
}

#[inline(always)]
pub(crate) fn expand_packed<F>(buf: &mut [u8], channels: usize, bit_depth: u8, mut func: F)
where
    F: FnMut(u8, &mut [u8]),
{
    let pixels = buf.len() / channels * bit_depth as usize;
    let extra = pixels % 8;
    let entries = pixels / 8
        + match extra {
            0 => 0,
            _ => 1,
        };
    let mask = ((1u16 << bit_depth) - 1) as u8;
    let i = (0..entries)
        .rev() // Reverse iterator
        .flat_map(|idx|
            // This has to be reversed to
            range_step(0, 8, bit_depth)
            .zip(repeat(idx)))
        .skip(extra);
    let channels = channels as isize;
    let j = range_step(buf.len() as isize - channels, -channels, -channels);
    //let j = range_step(0, buf.len(), channels).rev(); // ideal solution;
    for ((shift, i), j) in i.zip(j) {
        let pixel = (buf[i] & (mask << shift)) >> shift;
        func(pixel, &mut buf[j as usize..(j + channels) as usize])
    }
}

/// Expand a buffer of packed 1, 2, or 4 bits integers into u8's. Assumes that
/// every `row_size` entries there are padding bits up to the next byte boundry.
#[allow(dead_code)]
// When no image formats that use it are enabled
pub(crate) fn expand_bits(bit_depth: u8, row_size: u32, buf: &[u8]) -> Vec<u8> {
    // Note: this conversion assumes that the scanlines begin on byte boundaries
    let mask = (1u8 << bit_depth as usize) - 1;
    let scaling_factor = 255 / ((1 << bit_depth as usize) - 1);
    let bit_width = row_size * u32::from(bit_depth);
    let skip = if bit_width % 8 == 0 {
        0
    } else {
        (8 - bit_width % 8) / u32::from(bit_depth)
    };
    let row_len = row_size + skip;
    let mut p = Vec::new();
    let mut i = 0;
    for v in buf {
        for shift in num_iter::range_step_inclusive(8i8 - (bit_depth as i8), 0, -(bit_depth as i8))
        {
            // skip the pixels that can be neglected because scanlines should
            // start at byte boundaries
            if i % (row_len as usize) < (row_size as usize) {
                let pixel = (v & mask << shift as usize) >> shift as usize;
                p.push(pixel * scaling_factor);
            }
            i += 1;
        }
    }
    p
}

/// Checks if the provided dimensions would cause an overflow.
#[allow(dead_code)]
// When no image formats that use it are enabled
pub(crate) fn check_dimension_overflow(width: u32, height: u32, bytes_per_pixel: u8) -> bool {
    width as u64 * height as u64 > std::u64::MAX / bytes_per_pixel as u64
}

#[allow(dead_code)]
// When no image formats that use it are enabled
pub(crate) fn vec_copy_to_u8<T>(vec: &[T]) -> Vec<u8>
where
    T: bytemuck::Pod,
{
    bytemuck::cast_slice(vec).to_owned()
}

#[inline]
pub(crate) fn clamp<N>(a: N, min: N, max: N) -> N
where
    N: PartialOrd,
{
    if a < min {
        min
    } else if a > max {
        max
    } else {
        a
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn gray_to_luma8_skip() {
        let check = |bit_depth, w, from, to| {
            assert_eq!(super::expand_bits(bit_depth, w, from), to);
        };
        // Bit depth 1, skip is more than half a byte
        check(
            1,
            10,
            &[0b11110000, 0b11000000, 0b00001111, 0b11000000],
            vec![
                255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        );
        // Bit depth 2, skip is more than half a byte
        check(
            2,
            5,
            &[0b11110000, 0b11000000, 0b00001111, 0b11000000],
            vec![255, 255, 0, 0, 255, 0, 0, 255, 255, 255],
        );
        // Bit depth 2, skip is 0
        check(
            2,
            4,
            &[0b11110000, 0b00001111],
            vec![255, 255, 0, 0, 0, 0, 255, 255],
        );
        // Bit depth 4, skip is half a byte
        check(4, 1, &[0b11110011, 0b00001100], vec![255, 0]);
    }
}
