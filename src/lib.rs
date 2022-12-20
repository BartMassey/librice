use bitstream_io::*;

pub struct Codec(u32);

impl Codec {
    pub fn encode_word<T, W: BitWrite>(&self, src: T, w: &mut W) -> std::io::Result<()>
    where
        T: Numeric + Into<u32>,
    {
        let k = self.0;
        let mut high = src.into() >> k;
        if high + k + 3 > T::BITS_SIZE {
            w.write_bit(false)?;
            w.write(T::BITS_SIZE, src)?;
        } else {
            w.write_bit(true)?;
            while high > 32 {
                w.write::<u32>(32, 0xffffffff)?;
                high -= 32;
            }
            w.write::<u32>(high, (1 << high) - 1)?;
            w.write_bit(false)?;
            let rem = src.into() & ((1 << k) - 1);
            w.write::<u32>(k, rem)?;
        }
        Ok(())
    }

    pub fn decode_word<T, R: BitRead>(&self, r: &mut R) -> std::io::Result<T>
    where
        T: Numeric + Into<u32>,
        u32: TryInto<T>,
        <u32 as TryInto<T>>::Error: std::fmt::Debug,
    {
        let k = self.0;
        let mut result = 0;
        if r.read_bit()? {
            while r.read_bit()? {
                result += 1;
            }
            result <<= k;
            let rem = r.read::<u32>(k)?;
            result |= rem;
        } else {
            result = r.read(T::BITS_SIZE)?;
        }
        Ok(result.try_into().unwrap())
    }
}

#[test]
fn test_roundtrip() {
    use std::io::Cursor;

    let codec = Codec(3);
    for i in 0u8..=255 {
        let mut buf = [0u8; 4];

        let c = Cursor::new(buf.as_mut());
        let mut b: BitWriter<_, BE> = BitWriter::new(c);
        codec.encode_word(i, &mut b).unwrap();
        b.write::<u32>(8, 0).unwrap();

        let mut c = b.into_writer();
        c.set_position(0);
        if i < 8 {
            assert_eq!(&[0b10000_000 + (i << 3), 0, 0, 0], c.get_ref());
        }

        let mut b: BitReader<_, BE> = BitReader::new(c);
        let j = codec.decode_word(&mut b).unwrap();

        assert_eq!(i, j);
    }
}
