//! Simple byte packing, for reducing the size of a sequence of
//! bytes that contain continuous sequences of `0`s.

use crate::ser_de::ByteViewer;

/// The delimiter that indicates a packed sequence of zeroes.
const BYTE_COUNT_DELIM: u8 = '#' as u8;

/// Pack a sequence of bytes
pub fn pack_bytes(input: &[u8]) -> Vec<u8> {
    let mut viewer = ByteViewer::from_slice(input);

    // max vec len is current slice size
    let mut packed = Vec::with_capacity(input.len());

    loop {
        // println!("view: {:?}", viewer.curr_iter().collect::<Vec<_>>());

        // looks for any more zeroes
        match viewer.find_byte(0) {
            Some(offset) => {
                // println!("offset to next zero byte: {}", offset);
                // add non matching bits
                packed.extend(viewer.next_bytes(offset, true));
            }
            None => {
                match viewer.is_end() {
                    true => (),
                    false => {
                        packed.extend(viewer.curr_iter());
                        viewer.advance(viewer.distance_to_end()).unwrap();
                    }
                }

                break;
            }
        }

        let num_zeroes = viewer.num_duplicates();
        // println!("num zeroes: {}", num_zeroes);

        match num_zeroes {
            // skip, do not pack
            0..=3 => packed.extend(viewer.next_bytes(num_zeroes, true)),
            // proceed
            4..=255 => {
                // create and push the marker
                let marker = [BYTE_COUNT_DELIM, num_zeroes as u8, BYTE_COUNT_DELIM];
                packed.extend(marker);
                viewer.advance(num_zeroes).unwrap();
            }
            // for sequences larger than u8::MAX
            _ => {
                let marker = [BYTE_COUNT_DELIM, u8::MAX as u8, BYTE_COUNT_DELIM];
                packed.extend(marker);
                viewer.advance(u8::MAX as usize).unwrap();
            }
        }

        // println!("current vec: {:?}", &packed);
    }

    packed
}

/// Unpack a packed sequence of bytess
pub fn unpack_bytes(input: &[u8]) -> Vec<u8> {
    let mut viewer = ByteViewer::from_slice(input);

    let mut unpacked = Vec::with_capacity(2 * input.len());

    // search for and expand any delimited packed sequence
    while !viewer.is_end() {
        // println!(
        //     "bytes left in view: {} - {:?}",
        //     viewer.distance_to_end(),
        //     viewer.curr_iter().collect::<Vec<_>>()
        // );

        match viewer.distance_to_end() {
            // stop condition, push the rest
            0..=2 => {
                unpacked.extend(viewer.curr_iter());
                viewer.advance(viewer.distance_to_end()).unwrap();
                break;
            }
            _ => (),
        }

        let window = viewer.next_bytes_fixed::<3>(false);

        match window {
            [BYTE_COUNT_DELIM, count, BYTE_COUNT_DELIM] => {
                let expanded = [0_u8].repeat(count as usize);
                unpacked.extend(expanded);
                viewer.advance(3).unwrap();
            }
            _ => unpacked.push(viewer.next_byte()),
        }
    }

    unpacked
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_pack_bytes() {
        let bytes = vec![1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 3, 10, 0, 0, 0, 0];

        let mut view = ByteViewer::from_slice(&bytes);
        view.advance(view.distance_to_end()).unwrap();

        let packed = pack_bytes(&bytes);

        println!("{:?}", bytes);
        println!("{:?}", packed);

        let unpacked = unpack_bytes(&packed);

        println!("{:?}", unpacked);

        assert_eq!(bytes, unpacked);
    }

    /// Test the packer on 0-sequences greater than `u8::MAX`
    #[test]
    fn test_pack_arbitrary_len_bytes() {
        let bytes = (0..1000).into_iter().map(|_| 0_u8).collect::<Vec<_>>();

        let packed = pack_bytes(&bytes);

        println!("packed contents: {:?}", packed);

        let unpacked = unpack_bytes(&packed);

        assert_eq!(bytes, unpacked);
    }
}
