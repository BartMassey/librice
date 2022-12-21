use std::ops::*;

pub use bitstream_io;
use bitstream_io::*;
use easy_cast::*;

pub struct Codec(pub u32);

fn as_big<T>(value: T, target: u32) -> bool
where
    u32: Conv<T>,
{
    let downgrade: Result<u32> = value.try_cast();
    if let Ok(value) = downgrade {
        value >= target
    } else {
        true
    }
}

#[test]
fn test_as_big() {
    assert!(as_big(0u8, 0));
    assert!(!as_big(0u8, 1));
    assert!(as_big(1u8, 0));
    assert!(!as_big(1u32 << 31 - 1, 1u32 << 31));
    assert!(as_big(1u32 << 31, 1u32 << 31));
    assert!(as_big(1u64 << 63, 1u32 << 31));
}

fn mask<T: Numeric>(nbits: u32) -> T
where
    u32: Cast<T>,
{
    assert!(nbits <= T::BITS_SIZE);
    if nbits == T::BITS_SIZE {
        return !(0.cast());
    }
    (T::ONE << nbits) - T::ONE
}

#[test]
fn test_mask() {
    assert_eq!(0, mask::<u32>(0));
    assert_eq!(1, mask::<u32>(1));
    assert_eq!(0b11, mask::<u32>(2));
    assert_eq!(0b111, mask::<u32>(3));
    assert_eq!(0xffffffff, mask::<u32>(32));
}

impl Codec {
    pub fn encode_word<T, W: BitWrite>(&self, src: T, w: &mut W) -> std::io::Result<()>
    where
        T: Numeric + SubAssign + Add<Output = T> + BitAnd<Output = T> + Conv<u32>,
        u32: Conv<T>,
    {
        let k = self.0;
        let mut high = src >> k;
        let compressable = !as_big(high + k.cast() + 2.cast(), T::BITS_SIZE + 1);
        if compressable {
            let step = 32;

            w.write_bit(true)?;

            while as_big(high, step) {
                w.write::<T>(step, mask(step))?;
                high -= step.cast();
            }
            let high = high.cast();
            let high_mask: u32 = mask(high);
            w.write(high, high_mask)?;

            w.write_bit(false)?;

            let low_mask: T = mask(k);
            w.write(k, src & low_mask)?;
        } else {
            w.write_bit(false)?;
            w.write(T::BITS_SIZE, src)?;
        }
        Ok(())
    }

    pub fn decode_word<T, R: BitRead>(&self, r: &mut R) -> std::io::Result<T>
    where
        T: Numeric + AddAssign + Conv<u32>,
    {
        let k = self.0;
        let mut result = 0u32.cast();
        if r.read_bit()? {
            while r.read_bit()? {
                result += T::ONE;
            }
            result <<= k;
            let rem = r.read(k)?;
            result |= rem;
        } else {
            result = r.read(T::BITS_SIZE)?;
        }
        Ok(result)
    }
}

#[test]
fn test_roundtrip() {
    use std::io::Cursor;

    let codec = Codec(3);
    for i in 0u8..=255 {
        let mut buf = [0u8; 8];

        let c = Cursor::new(buf.as_mut());
        let mut b: BitWriter<_, BE> = BitWriter::new(c);
        codec.encode_word(i, &mut b).unwrap();
        b.write::<u32>(8, 0).unwrap();

        let mut c = b.into_writer();
        c.set_position(0);
        if i < 8 {
            assert_eq!(&[0b10000_000 + (i << 3), 0, 0, 0, 0, 0, 0, 0], c.get_ref());
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
