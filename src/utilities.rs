pub fn modular_distance(a0: f64, b0: f64, modulus: f64) -> f64 {
    let (a, b) = if a0 < b0 { (a0, b0) } else { (b0, a0) };
    f64::min(b - a, a + modulus - b)
}

pub fn positive_mod(x: f64, modulus: f64) -> f64 {
    if x < 0.0 {
        x + (x.abs() / modulus).ceil() * modulus
    } else {
        x % modulus
    }
}

pub fn erf_approximation(x: f64) -> f64 {
    const A1: f64 = 0.278393;
    const A2: f64 = 0.230389;
    const A3: f64 = 0.000972;
    const A4: f64 = 0.078108;

    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x3 * x;

    let q = 1.0 / (1.0 + A1 * x + A2 * x2 + A3 * x3 + A4 * x4);
    let q2 = q * q;
    let q4 = q2 * q2;

    1.0 - q4
}

pub fn clamp<T: PartialOrd>(x: T, x0: T, x1: T) -> T {
    if x <= x0 {
        x0
    } else if x >= x1 {
        x1
    } else {
        x
    }
}
