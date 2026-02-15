const fn xtime(x: u8) -> u8 {
    if x & 0x80 != 0 {
        (x << 1) ^ 0x1B
    } else {
        x << 1
    }
}

const fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut result = 0u8;
    while b != 0 {
        if b & 1 != 0 {
            result ^= a;
        }
        a = xtime(a);
        b >>= 1;
    }
    result
}

const LOG: [u8; 256] = {
    let mut log = [0u8; 256];
    let mut x: u8 = 1;
    let mut i: u8 = 0;
    loop {
        log[x as usize] = i;
        x = gf_mul(x, 3);
        if i == 254 {
            break;
        }
        i += 1;
    }
    log
};

const EXP: [u8; 512] = {
    let mut exp = [0u8; 512];
    let mut x: u8 = 1;
    let mut i: usize = 0;
    while i < 255 {
        exp[i] = x;
        exp[i + 255] = x;
        x = gf_mul(x, 3);
        i += 1;
    }
    exp
};

#[inline]
pub fn add(a: u8, b: u8) -> u8 {
    a ^ b
}

#[inline]
pub fn sub(a: u8, b: u8) -> u8 {
    a ^ b
}

#[inline]
pub fn mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let log_a = LOG[a as usize] as usize;
    let log_b = LOG[b as usize] as usize;
    EXP[log_a + log_b]
}

#[inline]
pub fn div(a: u8, b: u8) -> Option<u8> {
    if b == 0 {
        return None;
    }
    if a == 0 {
        return Some(0);
    }
    let log_a = LOG[a as usize] as usize;
    let log_b = LOG[b as usize] as usize;
    let diff = if log_a >= log_b {
        log_a - log_b
    } else {
        255 + log_a - log_b
    };
    Some(EXP[diff])
}

#[cfg(test)]
#[inline]
fn inv(a: u8) -> Option<u8> {
    if a == 0 {
        return None;
    }
    let log_a = LOG[a as usize] as usize;
    Some(EXP[255 - log_a])
}

pub fn eval_poly(coefficients: &[u8], x: u8) -> u8 {
    if coefficients.is_empty() {
        return 0;
    }
    let mut result = coefficients[coefficients.len() - 1];
    for i in (0..coefficients.len() - 1).rev() {
        result = add(mul(result, x), coefficients[i]);
    }
    result
}

pub fn interpolate_at_zero(points: &[(u8, u8)]) -> Option<u8> {
    let mut result: u8 = 0;

    for (i, &(xi, yi)) in points.iter().enumerate() {
        let mut basis: u8 = 1;

        for (j, &(xj, _)) in points.iter().enumerate() {
            if i != j {
                let numerator = xj;
                let denominator = sub(xj, xi);
                basis = mul(basis, div(numerator, denominator)?);
            }
        }

        result = add(result, mul(yi, basis));
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gf256_basic() {
        assert_eq!(add(0x53, 0xCA), 0x53 ^ 0xCA);
        assert_eq!(mul(0, 0xCA), 0);
        assert_eq!(mul(1, 0xCA), 0xCA);
        assert_eq!(mul(0xCA, 1), 0xCA);

        for a in [5u8, 17, 83, 202, 255] {
            for b in [3u8, 7, 13, 100, 200] {
                let product = mul(a, b);
                assert_eq!(div(product, b), Some(a), "div(mul({}, {}), {}) != {}", a, b, b, a);
            }
        }

        for a in [2u8, 3, 53, 83, 127, 200, 255] {
            assert_eq!(mul(a, inv(a).unwrap()), 1, "mul({}, inv({})) != 1", a, a);
        }
    }

    #[test]
    fn test_polynomial_eval() {
        let coeffs = [3u8, 2, 1];
        let result = eval_poly(&coeffs, 2);
        let expected = add(add(3, mul(2, 2)), mul(1, mul(2, 2)));
        assert_eq!(result, expected);
    }

    #[test]
    fn test_interpolation() {
        let secret = 42u8;
        let a1 = 17u8;

        let y1 = add(secret, mul(a1, 1));
        let y2 = add(secret, mul(a1, 2));

        let points = [(1u8, y1), (2u8, y2)];
        let recovered = interpolate_at_zero(&points).unwrap();

        assert_eq!(recovered, secret);
    }

    #[test]
    fn test_gf256_inverse() {
        for a in 1..=255u8 {
            let a_inv = inv(a).unwrap();
            let product = mul(a, a_inv);
            assert_eq!(product, 1, "inv({}) * {} should equal 1, got {}", a, a, product);
        }
    }

    #[test]
    fn test_div_by_zero() {
        assert_eq!(div(5, 0), None);
        assert_eq!(div(0, 0), None);
    }

    #[test]
    fn test_inv_zero() {
        assert_eq!(inv(0), None);
    }

    #[test]
    fn test_gf256_div_uses_inv() {
        for a in 1..=50u8 {
            for b in 1..=50u8 {
                let quotient = div(a, b).unwrap();
                let product = mul(a, inv(b).unwrap());
                assert_eq!(quotient, product, "{} / {} via div vs mul*inv", a, b);
            }
        }
    }
}
