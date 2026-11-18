//! Choose distinct, human-readable integer tick positions.

pub fn ticks(start: u64, end: u64, max_labels: usize) -> Vec<u64> {
    if start > end || max_labels == 0 {
        return Vec::new();
    }
    if max_labels == 1 || start == end {
        return vec![start];
    }
    let desired = u128::from(end - start)
        .div_ceil((max_labels - 1) as u128)
        .max(1);
    let mut power = 1u128;
    while power * 10 < desired {
        power *= 10;
    }
    let step = [1, 2, 5, 10]
        .into_iter()
        .map(|n| n * power)
        .find(|&n| n >= desired)
        .unwrap_or(power * 10);
    let mut tick = u128::from(start).div_ceil(step) * step;
    let mut output = Vec::new();
    while tick <= u128::from(end) && output.len() < max_labels {
        output.push(tick as u64);
        tick += step;
    }
    if output.is_empty() {
        output.push(start);
        if start < end {
            output.push(end);
        }
    }
    output
}
