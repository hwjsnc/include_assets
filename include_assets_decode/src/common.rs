use crate::codec::Codec;

const _I_DONT_CARE_ABOUT_16_BIT_TARGETS: () = if std::mem::size_of::<usize>() < 4 {
    panic!("yeah, no, this ain't gonna fly")
} else {
};

#[inline(always)]
pub fn u32_to_usize(x: u32) -> usize {
    x as _
}

#[inline(always)]
pub fn u32_to_usize_range<R: core::borrow::Borrow<core::ops::Range<u32>>>(range: R) -> core::ops::Range<usize> {
    u32_to_usize(range.borrow().start)..u32_to_usize(range.borrow().end)
}

pub fn decompress_ranges<C: Codec>(codec: &C, compressed_lengths: &[u8], number_of_entries: usize) -> std::vec::Vec<core::ops::Range<u32>> {
    let decompressed_len = number_of_entries
        .checked_mul(4)
        .expect("multiplication should not overflow at runtime because it would have overflowed at compile time already");
    let decompressed_lengths = codec.decompress_with_length(compressed_lengths, decompressed_len);
    let mut ranges = std::vec::Vec::<std::ops::Range<u32>>::with_capacity(number_of_entries);
    for slice in decompressed_lengths.chunks(4) {
        let len = u32::from_le_bytes(slice.try_into().expect("length is divisible by 4"));
        let start = ranges.last().map(|range| range.end).unwrap_or(0);
        let end = start
            .checked_add(len)
            .expect("overflow should have been caught during construction at compile time");
        ranges.push(start..end);
    }
    ranges
}

pub fn decompress_names<C: Codec>(
    codec: &C,
    compressed_names_with_null_delimiters: &[u8],
    decompressed_len: u32,
) -> std::vec::Vec<smartstring::SmartString<smartstring::LazyCompact>> {
    let decompressed_data = codec.decompress_with_length(compressed_names_with_null_delimiters, u32_to_usize(decompressed_len));
    let names = decompressed_data.split(|b| *b == 0);
    names.map(|bytes| std::str::from_utf8(bytes).expect("names should be UTF-8").into()).collect()
}
