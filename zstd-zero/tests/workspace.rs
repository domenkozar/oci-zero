use zstd_zero::{
    Decoder, DecoderBuffers, FseEntry, HuffmanEntry, FSE_ENTRIES, HUFFMAN_ENTRIES, MAX_BLOCK_SIZE,
};

#[test]
fn entropy_storage_survives_reset_poison_and_reconstruction() {
    let payloads = [
        b"first alphabet and repeating sequence".repeat(2000),
        (0..64000).map(|i| ((i * 7 + i / 17) % 31) as u8).collect(),
    ];
    let frames: Vec<_> = payloads
        .iter()
        .map(|p| zstd::bulk::compress(p, 9).unwrap())
        .collect();
    let mut history = vec![0; MAX_BLOCK_SIZE];
    let mut block = vec![0; MAX_BLOCK_SIZE];
    let mut literals = vec![0; MAX_BLOCK_SIZE];
    // Extra storage is allowed and can be reused after dropping the decoder.
    let mut fse = vec![FseEntry::new(); FSE_ENTRIES + 1];
    let mut huffman = vec![HuffmanEntry::new(); HUFFMAN_ENTRIES + 1];
    for _ in 0..2 {
        let mut decoder = Decoder::new(DecoderBuffers {
            history: &mut history,
            block: &mut block,
            literals: &mut literals,
            fse: &mut fse,
            huffman: &mut huffman,
        })
        .unwrap();
        for (frame, expected) in frames.iter().zip(&payloads) {
            decoder.reset();
            let mut output = Vec::new();
            for part in frame.chunks(7) {
                decoder
                    .push(part, |bytes| {
                        output.extend_from_slice(bytes);
                        Ok::<_, ()>(())
                    })
                    .unwrap();
            }
            decoder
                .finish_with(|bytes| {
                    output.extend_from_slice(bytes);
                    Ok::<_, ()>(())
                })
                .unwrap();
            assert_eq!(&output, expected);
            decoder.reset();
            assert!(decoder.decode(b"not zstd").is_err());
        }
    }
}

#[test]
fn decoder_does_not_embed_entropy_workspaces() {
    // A stack-safety regression guard, not a stable public layout promise.
    assert!(std::mem::size_of::<Decoder<'_>>() <= 1024);
}
