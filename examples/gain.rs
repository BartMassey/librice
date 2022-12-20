use rice::{bitstream_io::*, *};

fn main() {
    use rand::*;
    use std::io::Cursor;

    let mut rng = rand::thread_rng();

    for k in 1..13 {
        let msg: [u16; 32] = std::array::from_fn(|_| rng.gen_range(0..(1 << (k + 3))));

        let codec = Codec(k);
        let c = Cursor::new(Vec::new());

        let mut b: BitWriter<_, BE> = BitWriter::new(c);
        for w in msg {
            codec.encode_word(w, &mut b).unwrap();
        }
        b.write::<u32>(16, 0).unwrap();

        let mut c = b.into_writer();
        println!("k = {k}, ratio = {}", c.get_ref().len() as f64 / 64.0);
        c.set_position(0);

        let mut r: BitReader<_, BE> = BitReader::new(c);
        let dmsg: Vec<u16> = (0..32)
            .map(|_| codec.decode_word(&mut r).unwrap())
            .collect();
        assert_eq!(msg.as_ref(), &dmsg);
    }
}
