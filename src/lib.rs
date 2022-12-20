pub use bitstream_io;
use bitstream_io::*;

pub struct Codec(pub u32);

impl Codec {
    pub fn encode_word<T, W: BitWrite>(&self, src: T, w: &mut W) -> std::io::Result<()>
    where
        T: Numeric + Into<u32>,
    {
        let k = self.0;
        let mut high = src.into() >> k;
        if high + k + 2 > T::BITS_SIZE + 1 {
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

#[test]
fn test_random() {
    use rand::*;
    use std::io::Cursor;

    let mut rng = rand::thread_rng();
    let msg: [u16; 32] = std::array::from_fn(|_| rng.gen());

    for k in 1..13 {
        let codec = Codec(k);
        let c = Cursor::new(Vec::new());

        let mut b: BitWriter<_, BE> = BitWriter::new(c);
        for w in msg {
            codec.encode_word(w, &mut b).unwrap();
        }
        b.write::<u32>(16, 0).unwrap();

        let mut c = b.into_writer();
        c.set_position(0);

        let mut r: BitReader<_, BE> = BitReader::new(c);
        let dmsg: Vec<u16> = (0..32)
            .map(|_| codec.decode_word(&mut r).unwrap())
            .collect();
        assert_eq!(msg.as_ref(), &dmsg);
    }
}
