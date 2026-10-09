//! Java `util/Simplex.java` — OpenSimplex2 (K.jpg faster variant).
//!
//! Ported 1:1 from Minicraft+ 2.2.4. Java `float` maps to Rust `f32`,
//! `long` arithmetic is wrapping `i64`. `noise3*` and `noise4*` temporarily
//! mutate the instance seed and restore it, matching Java field mutation.

// Keep Java literals intact: shortening f32 constants or substituting constants
// can change deterministic noise rounding.
#![allow(clippy::excessive_precision, clippy::approx_constant)]

use std::sync::OnceLock;

// A bunch of funny magic numbers
const PRIME_X: i64 = 0x5205_402B_9270_C86F_u64 as i64;
const PRIME_Y: i64 = 0x598C_D327_0038_17B5_u64 as i64;
const PRIME_Z: i64 = 0x5BCC_226E_9FA0_BACB_u64 as i64;
const PRIME_W: i64 = 0x56CC_5227_E58F_554B_u64 as i64;
const HASH_MULTIPLIER: i64 = 0x53A3_F72D_EEC5_46F5_u64 as i64;
const SEED_FLIP_3D: i64 = -0x52D5_47B2_E96E_D629_i64;
const SEED_OFFSET_4D: i64 = 0x0E83_DC3E_0DA7_164D_u64 as i64;

const ROOT2OVER2: f64 = 0.7071067811865476;
const SKEW_2D: f64 = 0.366025403784439;
const UNSKEW_2D: f64 = -0.21132486540518713;

const ROOT3OVER3: f64 = 0.577350269189626;
const FALLBACK_ROTATE_3D: f64 = 2.0 / 3.0;
const ROTATE_3D_ORTHOGONALIZER: f64 = UNSKEW_2D;

const SKEW_4D: f32 = -0.138196601125011;
const UNSKEW_4D: f32 = 0.309016994374947;
const LATTICE_STEP_4D: f32 = 0.2;

const N_GRADS_2D_EXPONENT: i32 = 7;
const N_GRADS_3D_EXPONENT: i32 = 8;
const N_GRADS_4D_EXPONENT: i32 = 9;
const N_GRADS_2D: i32 = 1 << N_GRADS_2D_EXPONENT;
const N_GRADS_3D: i32 = 1 << N_GRADS_3D_EXPONENT;
const N_GRADS_4D: i32 = 1 << N_GRADS_4D_EXPONENT;

const NORMALIZER_2D: f64 = 0.01001634121365712;
const NORMALIZER_3D: f64 = 0.07969837668935331;
const NORMALIZER_4D: f64 = 0.0220065933241897;

const RSQUARED_2D: f32 = 0.5;
const RSQUARED_3D: f32 = 0.6;
const RSQUARED_4D: f32 = 0.6;

/// OpenSimplex2 noise generator matching Java `minicraft.util.Simplex`.
#[derive(Debug, Clone)]
pub struct Simplex {
    seed: i64,
}

impl Simplex {
    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    pub fn set_seed(&mut self, seed: i64) {
        self.seed = seed;
    }

    pub fn seed(&self) -> i64 {
        self.seed
    }

    /*
     * Noise Evaluators
     */

    /// 2D Simplex noise, standard lattice orientation.
    pub fn noise2(&self, x: f64, y: f64) -> f32 {
        let s = SKEW_2D * (x + y);
        let xs = x + s;
        let ys = y + s;
        self.noise2_unskewed_base(xs, ys)
    }

    /// 2D Simplex noise, with Y pointing down the main diagonal.
    pub fn noise2_improve_x(&self, x: f64, y: f64) -> f32 {
        let xx = x * ROOT2OVER2;
        let yy = y * (ROOT2OVER2 * (1.0 + 2.0 * SKEW_2D));
        self.noise2_unskewed_base(yy + xx, yy - xx)
    }

    fn noise2_unskewed_base(&self, xs: f64, ys: f64) -> f32 {
        let xsb = fast_floor(xs);
        let ysb = fast_floor(ys);
        let xi = (xs - xsb as f64) as f32;
        let yi = (ys - ysb as f64) as f32;

        let xsbp = (xsb as i64).wrapping_mul(PRIME_X);
        let ysbp = (ysb as i64).wrapping_mul(PRIME_Y);

        let t = (xi + yi) * (UNSKEW_2D as f32);
        let dx0 = xi + t;
        let dy0 = yi + t;

        let mut value: f32 = 0.0;
        let a0 = RSQUARED_2D - dx0 * dx0 - dy0 * dy0;
        if a0 > 0.0 {
            value = (a0 * a0) * (a0 * a0) * self.grad2(xsbp, ysbp, dx0, dy0);
        }

        let a1 = (2.0 * (1.0 + 2.0 * UNSKEW_2D) * (1.0 / UNSKEW_2D + 2.0)) as f32 * t
            + ((-2.0 * (1.0 + 2.0 * UNSKEW_2D) * (1.0 + 2.0 * UNSKEW_2D)) as f32 + a0);
        if a1 > 0.0 {
            let dx1 = dx0 - (1.0 + 2.0 * UNSKEW_2D) as f32;
            let dy1 = dy0 - (1.0 + 2.0 * UNSKEW_2D) as f32;
            value += (a1 * a1)
                * (a1 * a1)
                * self.grad2(
                    xsbp.wrapping_add(PRIME_X),
                    ysbp.wrapping_add(PRIME_Y),
                    dx1,
                    dy1,
                );
        }

        if dy0 > dx0 {
            let dx2 = dx0 - UNSKEW_2D as f32;
            let dy2 = dy0 - (UNSKEW_2D + 1.0) as f32;
            let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
            if a2 > 0.0 {
                value +=
                    (a2 * a2) * (a2 * a2) * self.grad2(xsbp, ysbp.wrapping_add(PRIME_Y), dx2, dy2);
            }
        } else {
            let dx2 = dx0 - (UNSKEW_2D + 1.0) as f32;
            let dy2 = dy0 - UNSKEW_2D as f32;
            let a2 = RSQUARED_2D - dx2 * dx2 - dy2 * dy2;
            if a2 > 0.0 {
                value +=
                    (a2 * a2) * (a2 * a2) * self.grad2(xsbp.wrapping_add(PRIME_X), ysbp, dx2, dy2);
            }
        }

        value
    }

    /// Default noise3 (aliases `noise3_ImproveXY`).
    pub fn noise3(&mut self, x: f64, y: f64, z: f64) -> f32 {
        self.noise3_improve_xy(x, y, z)
    }

    /// 3D OpenSimplex2 noise, with better visual isotropy in (X, Y).
    pub fn noise3_improve_xy(&mut self, x: f64, y: f64, z: f64) -> f32 {
        let xy = x + y;
        let s2 = xy * ROTATE_3D_ORTHOGONALIZER;
        let zz = z * ROOT3OVER3;
        let xr = x + s2 + zz;
        let yr = y + s2 + zz;
        let zr = xy * -ROOT3OVER3 + zz;
        self.noise3_unrotated_base(xr, yr, zr)
    }

    /// 3D OpenSimplex2 noise, with better visual isotropy in (X, Z).
    pub fn noise3_improve_xz(&mut self, x: f64, y: f64, z: f64) -> f32 {
        let xz = x + z;
        let s2 = xz * ROTATE_3D_ORTHOGONALIZER;
        let yy = y * ROOT3OVER3;
        let xr = x + s2 + yy;
        let zr = z + s2 + yy;
        let yr = xz * -ROOT3OVER3 + yy;
        self.noise3_unrotated_base(xr, yr, zr)
    }

    /// 3D OpenSimplex2 noise, fallback rotation option.
    pub fn noise3_fallback(&mut self, x: f64, y: f64, z: f64) -> f32 {
        let r = FALLBACK_ROTATE_3D * (x + y + z);
        let xr = r - x;
        let yr = r - y;
        let zr = r - z;
        self.noise3_unrotated_base(xr, yr, zr)
    }

    fn noise3_unrotated_base(&mut self, xr: f64, yr: f64, zr: f64) -> f32 {
        let xrb = fast_round(xr);
        let yrb = fast_round(yr);
        let zrb = fast_round(zr);
        let mut xri = (xr - xrb as f64) as f32;
        let mut yri = (yr - yrb as f64) as f32;
        let mut zri = (zr - zrb as f64) as f32;

        let mut x_nsign = ((-1.0_f32 - xri) as i32) | 1;
        let mut y_nsign = ((-1.0_f32 - yri) as i32) | 1;
        let mut z_nsign = ((-1.0_f32 - zri) as i32) | 1;

        let mut ax0 = x_nsign as f32 * -xri;
        let mut ay0 = y_nsign as f32 * -yri;
        let mut az0 = z_nsign as f32 * -zri;

        let mut xrbp = (xrb as i64).wrapping_mul(PRIME_X);
        let mut yrbp = (yrb as i64).wrapping_mul(PRIME_Y);
        let mut zrbp = (zrb as i64).wrapping_mul(PRIME_Z);

        let mut value: f32 = 0.0;
        let mut a = (RSQUARED_3D - xri * xri) - (yri * yri + zri * zri);
        let p_seed = self.seed;
        for l in 0..=1 {
            if a > 0.0 {
                value += (a * a) * (a * a) * self.grad3(xrbp, yrbp, zrbp, xri, yri, zri);
            }

            if ax0 >= ay0 && ax0 >= az0 {
                let mut b = a + ax0 + ax0;
                if b > 1.0 {
                    b -= 1.0;
                    value += (b * b)
                        * (b * b)
                        * self.grad3(
                            xrbp.wrapping_sub((x_nsign as i64).wrapping_mul(PRIME_X)),
                            yrbp,
                            zrbp,
                            xri + x_nsign as f32,
                            yri,
                            zri,
                        );
                }
            } else if ay0 > ax0 && ay0 >= az0 {
                let mut b = a + ay0 + ay0;
                if b > 1.0 {
                    b -= 1.0;
                    value += (b * b)
                        * (b * b)
                        * self.grad3(
                            xrbp,
                            yrbp.wrapping_sub((y_nsign as i64).wrapping_mul(PRIME_Y)),
                            zrbp,
                            xri,
                            yri + y_nsign as f32,
                            zri,
                        );
                }
            } else {
                let mut b = a + az0 + az0;
                if b > 1.0 {
                    b -= 1.0;
                    value += (b * b)
                        * (b * b)
                        * self.grad3(
                            xrbp,
                            yrbp,
                            zrbp.wrapping_sub((z_nsign as i64).wrapping_mul(PRIME_Z)),
                            xri,
                            yri,
                            zri + z_nsign as f32,
                        );
                }
            }

            if l == 1 {
                break;
            }

            ax0 = 0.5 - ax0;
            ay0 = 0.5 - ay0;
            az0 = 0.5 - az0;

            xri = x_nsign as f32 * ax0;
            yri = y_nsign as f32 * ay0;
            zri = z_nsign as f32 * az0;

            a += (0.75 - ax0) - (ay0 + az0);

            xrbp = xrbp.wrapping_add(((x_nsign >> 1) as i64) & PRIME_X);
            yrbp = yrbp.wrapping_add(((y_nsign >> 1) as i64) & PRIME_Y);
            zrbp = zrbp.wrapping_add(((z_nsign >> 1) as i64) & PRIME_Z);

            x_nsign = -x_nsign;
            y_nsign = -y_nsign;
            z_nsign = -z_nsign;

            self.seed ^= SEED_FLIP_3D;
        }

        self.seed = p_seed;
        value
    }

    /*
     * 4D
     */

    pub fn noise4_improve_xyz_improve_xy(&mut self, x: f64, y: f64, z: f64, w: f64) -> f32 {
        let xy = x + y;
        let s2 = xy * -0.21132486540518699998;
        let zz = z * 0.28867513459481294226;
        let ww = w * 0.2236067977499788;
        let xr = x + (zz + ww + s2);
        let yr = y + (zz + ww + s2);
        let zr = xy * -0.57735026918962599998 + (zz + ww);
        let wr = z * -0.866025403784439 + ww;
        self.noise4_unskewed_base(xr, yr, zr, wr)
    }

    pub fn noise4_improve_xyz_improve_xz(&mut self, x: f64, y: f64, z: f64, w: f64) -> f32 {
        let xz = x + z;
        let s2 = xz * -0.21132486540518699998;
        let yy = y * 0.28867513459481294226;
        let ww = w * 0.2236067977499788;
        let xr = x + (yy + ww + s2);
        let zr = z + (yy + ww + s2);
        let yr = xz * -0.57735026918962599998 + (yy + ww);
        let wr = y * -0.866025403784439 + ww;
        self.noise4_unskewed_base(xr, yr, zr, wr)
    }

    pub fn noise4_improve_xyz(&mut self, x: f64, y: f64, z: f64, w: f64) -> f32 {
        let xyz = x + y + z;
        let ww = w * 0.2236067977499788;
        let s2 = xyz * -0.16666666666666666 + ww;
        let xs = x + s2;
        let ys = y + s2;
        let zs = z + s2;
        let ws = -0.5 * xyz + ww;
        self.noise4_unskewed_base(xs, ys, zs, ws)
    }

    pub fn noise4_improve_xy_improve_zw(&mut self, x: f64, y: f64, z: f64, w: f64) -> f32 {
        let s2 = (x + y) * -0.178275657951399372 + (z + w) * 0.215623393288842828;
        let t2 = (z + w) * -0.403949762580207112 + (x + y) * -0.375199083010075342;
        let xs = x + s2;
        let ys = y + s2;
        let zs = z + t2;
        let ws = w + t2;
        self.noise4_unskewed_base(xs, ys, zs, ws)
    }

    pub fn noise4_fallback(&mut self, x: f64, y: f64, z: f64, w: f64) -> f32 {
        let s = SKEW_4D as f64 * (x + y + z + w);
        let xs = x + s;
        let ys = y + s;
        let zs = z + s;
        let ws = w + s;
        self.noise4_unskewed_base(xs, ys, zs, ws)
    }

    fn noise4_unskewed_base(&mut self, xs: f64, ys: f64, zs: f64, ws: f64) -> f32 {
        let xsb = fast_floor(xs);
        let ysb = fast_floor(ys);
        let zsb = fast_floor(zs);
        let wsb = fast_floor(ws);
        let mut xsi = (xs - xsb as f64) as f32;
        let mut ysi = (ys - ysb as f64) as f32;
        let mut zsi = (zs - zsb as f64) as f32;
        let mut wsi = (ws - wsb as f64) as f32;

        let si_sum = (xsi + ysi) + (zsi + wsi);
        let starting_lattice = (si_sum * 1.25) as i32;

        let p_seed = self.seed;
        self.seed = self
            .seed
            .wrapping_add((starting_lattice as i64).wrapping_mul(SEED_OFFSET_4D));

        let starting_lattice_offset = starting_lattice as f32 * -LATTICE_STEP_4D;
        xsi += starting_lattice_offset;
        ysi += starting_lattice_offset;
        zsi += starting_lattice_offset;
        wsi += starting_lattice_offset;

        let mut ssi = (si_sum + starting_lattice_offset * 4.0) * UNSKEW_4D;

        let mut xsvp = (xsb as i64).wrapping_mul(PRIME_X);
        let mut ysvp = (ysb as i64).wrapping_mul(PRIME_Y);
        let mut zsvp = (zsb as i64).wrapping_mul(PRIME_Z);
        let mut wsvp = (wsb as i64).wrapping_mul(PRIME_W);

        let mut value: f32 = 0.0;
        for i in 0..=4 {
            let score0 = 1.0 + ssi as f64 * (-1.0 / UNSKEW_4D as f64);
            if xsi >= ysi && xsi >= zsi && xsi >= wsi && xsi as f64 >= score0 {
                xsvp = xsvp.wrapping_add(PRIME_X);
                xsi -= 1.0;
                ssi -= UNSKEW_4D;
            } else if ysi > xsi && ysi >= zsi && ysi >= wsi && ysi as f64 >= score0 {
                ysvp = ysvp.wrapping_add(PRIME_Y);
                ysi -= 1.0;
                ssi -= UNSKEW_4D;
            } else if zsi > xsi && zsi > ysi && zsi >= wsi && zsi as f64 >= score0 {
                zsvp = zsvp.wrapping_add(PRIME_Z);
                zsi -= 1.0;
                ssi -= UNSKEW_4D;
            } else if wsi > xsi && wsi > ysi && wsi > zsi && wsi as f64 >= score0 {
                wsvp = wsvp.wrapping_add(PRIME_W);
                wsi -= 1.0;
                ssi -= UNSKEW_4D;
            }

            let dx = xsi + ssi;
            let dy = ysi + ssi;
            let dz = zsi + ssi;
            let dw = wsi + ssi;
            let mut a = (dx * dx + dy * dy) + (dz * dz + dw * dw);
            if a < RSQUARED_4D {
                a -= RSQUARED_4D;
                a *= a;
                value += a * a * self.grad4(xsvp, ysvp, zsvp, wsvp, dx, dy, dz, dw);
            }

            if i == 4 {
                break;
            }

            xsi += LATTICE_STEP_4D;
            ysi += LATTICE_STEP_4D;
            zsi += LATTICE_STEP_4D;
            wsi += LATTICE_STEP_4D;
            ssi += LATTICE_STEP_4D * 4.0 * UNSKEW_4D;
            self.seed = self.seed.wrapping_sub(SEED_OFFSET_4D);

            if i == starting_lattice {
                xsvp = xsvp.wrapping_sub(PRIME_X);
                ysvp = ysvp.wrapping_sub(PRIME_Y);
                zsvp = zsvp.wrapping_sub(PRIME_Z);
                wsvp = wsvp.wrapping_sub(PRIME_W);
                self.seed = self.seed.wrapping_add(SEED_OFFSET_4D.wrapping_mul(5));
            }
        }

        self.seed = p_seed;
        value
    }

    /*
     * Utility
     */

    fn grad2(&self, xsvp: i64, ysvp: i64, dx: f32, dy: f32) -> f32 {
        let mut hash = self.seed ^ xsvp ^ ysvp;
        hash = hash.wrapping_mul(HASH_MULTIPLIER);
        hash ^= hash >> (64 - N_GRADS_2D_EXPONENT + 1);
        let gi = (hash as i32) & ((N_GRADS_2D - 1) << 1);
        let grads = gradients_2d();
        grads[gi as usize] * dx + grads[gi as usize + 1] * dy
    }

    fn grad3(&self, xrvp: i64, yrvp: i64, zrvp: i64, dx: f32, dy: f32, dz: f32) -> f32 {
        let mut hash = (self.seed ^ xrvp) ^ (yrvp ^ zrvp);
        hash = hash.wrapping_mul(HASH_MULTIPLIER);
        hash ^= hash >> (64 - N_GRADS_3D_EXPONENT + 2);
        let gi = (hash as i32) & ((N_GRADS_3D - 1) << 2);
        let grads = gradients_3d();
        grads[gi as usize] * dx + grads[gi as usize + 1] * dy + grads[gi as usize + 2] * dz
    }

    // Keep the existing coordinate/render API without adding wrapper state.
    #[allow(clippy::too_many_arguments)]
    fn grad4(
        &self,
        xsvp: i64,
        ysvp: i64,
        zsvp: i64,
        wsvp: i64,
        dx: f32,
        dy: f32,
        dz: f32,
        dw: f32,
    ) -> f32 {
        let mut hash = self.seed ^ (xsvp ^ ysvp) ^ (zsvp ^ wsvp);
        hash = hash.wrapping_mul(HASH_MULTIPLIER);
        hash ^= hash >> (64 - N_GRADS_4D_EXPONENT + 2);
        let gi = (hash as i32) & ((N_GRADS_4D - 1) << 2);
        let grads = gradients_4d();
        (grads[gi as usize] * dx + grads[gi as usize + 1] * dy)
            + (grads[gi as usize + 2] * dz + grads[gi as usize + 3] * dw)
    }
}

fn fast_floor(x: f64) -> i32 {
    let xi = x as i32;
    if x < xi as f64 { xi - 1 } else { xi }
}

fn fast_round(x: f64) -> i32 {
    if x < 0.0 {
        (x - 0.5) as i32
    } else {
        (x + 0.5) as i32
    }
}

static GRADIENTS_2D: OnceLock<Vec<f32>> = OnceLock::new();
static GRADIENTS_3D: OnceLock<Vec<f32>> = OnceLock::new();
static GRADIENTS_4D: OnceLock<Vec<f32>> = OnceLock::new();

fn gradients_2d() -> &'static [f32] {
    GRADIENTS_2D.get_or_init(|| {
        let mut grad2 = vec![
            0.38268343236509_f32,
            0.923879532511287,
            0.923879532511287,
            0.38268343236509,
            0.923879532511287,
            -0.38268343236509,
            0.38268343236509,
            -0.923879532511287,
            -0.38268343236509,
            -0.923879532511287,
            -0.923879532511287,
            -0.38268343236509,
            -0.923879532511287,
            0.38268343236509,
            -0.38268343236509,
            0.923879532511287,
            0.130526192220052,
            0.99144486137381,
            0.608761429008721,
            0.793353340291235,
            0.793353340291235,
            0.608761429008721,
            0.99144486137381,
            0.130526192220051,
            0.99144486137381,
            -0.130526192220051,
            0.793353340291235,
            -0.60876142900872,
            0.608761429008721,
            -0.793353340291235,
            0.130526192220052,
            -0.99144486137381,
            -0.130526192220052,
            -0.99144486137381,
            -0.608761429008721,
            -0.793353340291235,
            -0.793353340291235,
            -0.608761429008721,
            -0.99144486137381,
            -0.130526192220052,
            -0.99144486137381,
            0.130526192220051,
            -0.793353340291235,
            0.608761429008721,
            -0.608761429008721,
            0.793353340291235,
            -0.130526192220052,
            0.99144486137381,
        ];
        for value in grad2.iter_mut() {
            *value = (*value as f64 / NORMALIZER_2D) as f32;
        }
        let mut out = vec![0.0_f32; N_GRADS_2D as usize * 2];
        let mut j = 0;
        for slot in out.iter_mut() {
            if j == grad2.len() {
                j = 0;
            }
            *slot = grad2[j];
            j += 1;
        }
        out
    })
}

fn gradients_3d() -> &'static [f32] {
    GRADIENTS_3D.get_or_init(|| {
        let mut grad3 = vec![
            2.22474487139_f32,
            2.22474487139,
            -1.0,
            0.0,
            2.22474487139,
            2.22474487139,
            1.0,
            0.0,
            3.0862664687972017,
            1.1721513422464978,
            0.0,
            0.0,
            1.1721513422464978,
            3.0862664687972017,
            0.0,
            0.0,
            -2.22474487139,
            2.22474487139,
            -1.0,
            0.0,
            -2.22474487139,
            2.22474487139,
            1.0,
            0.0,
            -1.1721513422464978,
            3.0862664687972017,
            0.0,
            0.0,
            -3.0862664687972017,
            1.1721513422464978,
            0.0,
            0.0,
            -1.0,
            -2.22474487139,
            -2.22474487139,
            0.0,
            1.0,
            -2.22474487139,
            -2.22474487139,
            0.0,
            0.0,
            -3.0862664687972017,
            -1.1721513422464978,
            0.0,
            0.0,
            -1.1721513422464978,
            -3.0862664687972017,
            0.0,
            -1.0,
            -2.22474487139,
            2.22474487139,
            0.0,
            1.0,
            -2.22474487139,
            2.22474487139,
            0.0,
            0.0,
            -1.1721513422464978,
            3.0862664687972017,
            0.0,
            0.0,
            -3.0862664687972017,
            1.1721513422464978,
            0.0,
            -2.22474487139,
            -2.22474487139,
            -1.0,
            0.0,
            -2.22474487139,
            -2.22474487139,
            1.0,
            0.0,
            -3.0862664687972017,
            -1.1721513422464978,
            0.0,
            0.0,
            -1.1721513422464978,
            -3.0862664687972017,
            0.0,
            0.0,
            -2.22474487139,
            -1.0,
            -2.22474487139,
            0.0,
            -2.22474487139,
            1.0,
            -2.22474487139,
            0.0,
            -1.1721513422464978,
            0.0,
            -3.0862664687972017,
            0.0,
            -3.0862664687972017,
            0.0,
            -1.1721513422464978,
            0.0,
            -2.22474487139,
            -1.0,
            2.22474487139,
            0.0,
            -2.22474487139,
            1.0,
            2.22474487139,
            0.0,
            -3.0862664687972017,
            0.0,
            1.1721513422464978,
            0.0,
            -1.1721513422464978,
            0.0,
            3.0862664687972017,
            0.0,
            -1.0,
            2.22474487139,
            -2.22474487139,
            0.0,
            1.0,
            2.22474487139,
            -2.22474487139,
            0.0,
            0.0,
            1.1721513422464978,
            -3.0862664687972017,
            0.0,
            0.0,
            3.0862664687972017,
            -1.1721513422464978,
            0.0,
            -1.0,
            2.22474487139,
            2.22474487139,
            0.0,
            1.0,
            2.22474487139,
            2.22474487139,
            0.0,
            0.0,
            3.0862664687972017,
            1.1721513422464978,
            0.0,
            0.0,
            1.1721513422464978,
            3.0862664687972017,
            0.0,
            2.22474487139,
            -2.22474487139,
            -1.0,
            0.0,
            2.22474487139,
            -2.22474487139,
            1.0,
            0.0,
            1.1721513422464978,
            -3.0862664687972017,
            0.0,
            0.0,
            3.0862664687972017,
            -1.1721513422464978,
            0.0,
            0.0,
            2.22474487139,
            -1.0,
            -2.22474487139,
            0.0,
            2.22474487139,
            1.0,
            -2.22474487139,
            0.0,
            3.0862664687972017,
            0.0,
            -1.1721513422464978,
            0.0,
            1.1721513422464978,
            0.0,
            -3.0862664687972017,
            0.0,
            2.22474487139,
            -1.0,
            2.22474487139,
            0.0,
            2.22474487139,
            1.0,
            2.22474487139,
            0.0,
            3.0862664687972017,
            0.0,
            1.1721513422464978,
            0.0,
            1.1721513422464978,
            0.0,
            3.0862664687972017,
            0.0,
            2.22474487139,
            -1.0,
            -2.22474487139,
            0.0,
            2.22474487139,
            1.0,
            -2.22474487139,
            0.0,
            1.1721513422464978,
            0.0,
            -3.0862664687972017,
            0.0,
            3.0862664687972017,
            0.0,
            -1.1721513422464978,
            0.0,
            2.22474487139,
            -1.0,
            2.22474487139,
            0.0,
            2.22474487139,
            1.0,
            2.22474487139,
            0.0,
            1.1721513422464978,
            0.0,
            3.0862664687972017,
            0.0,
            3.0862664687972017,
            0.0,
            1.1721513422464978,
            0.0,
        ];
        for value in grad3.iter_mut() {
            *value = (*value as f64 / NORMALIZER_3D) as f32;
        }
        let mut out = vec![0.0_f32; N_GRADS_3D as usize * 4];
        let mut j = 0;
        for slot in out.iter_mut() {
            if j == grad3.len() {
                j = 0;
            }
            *slot = grad3[j];
            j += 1;
        }
        out
    })
}

fn gradients_4d() -> &'static [f32] {
    GRADIENTS_4D.get_or_init(|| {
        // Only the first 32*4 raw values are needed for the repeating pattern;
        // the full Java table is expanded by tiling these 32 quaternions
        // (Java's grad4 is 512*4 = 2048 floats, tiled from a 32*4 = 128-length
        // source after the first 32, then a second 32-entry block).
        // For bit-exactness we embed the full unique 32+32 = 64 quaternions
        // that Java tiles. Values below are the first block (normalized later).
        let raw = vec![
            -0.6740059517812944_f32,
            -0.3239847771997537,
            -0.3239847771997537,
            0.5794684678643381,
            -0.7504883828755602,
            -0.4004672082940195,
            0.15296486218853164,
            0.5029860367700724,
            -0.7504883828755602,
            0.15296486218853164,
            -0.4004672082940195,
            0.5029860367700724,
            -0.8828161875373585,
            0.08164729285680945,
            0.08164729285680945,
            0.4553054119602712,
            -0.4553054119602712,
            -0.08164729285680945,
            -0.08164729285680945,
            0.8828161875373585,
            -0.5029860367700724,
            -0.15296486218853164,
            0.4004672082940195,
            0.7504883828755602,
            -0.5029860367700724,
            0.4004672082940195,
            -0.15296486218853164,
            0.7504883828755602,
            -0.5794684678643381,
            0.3239847771997537,
            0.3239847771997537,
            0.6740059517812944,
            -0.6740059517812944,
            -0.3239847771997537,
            0.5794684678643381,
            -0.3239847771997537,
            -0.7504883828755602,
            -0.4004672082940195,
            0.5029860367700724,
            0.15296486218853164,
            -0.7504883828755602,
            0.15296486218853164,
            0.5029860367700724,
            -0.4004672082940195,
            -0.8828161875373585,
            0.08164729285680945,
            0.4553054119602712,
            0.08164729285680945,
            -0.4553054119602712,
            -0.08164729285680945,
            0.8828161875373585,
            -0.08164729285680945,
            -0.5029860367700724,
            -0.15296486218853164,
            0.7504883828755602,
            0.4004672082940195,
            -0.5029860367700724,
            0.4004672082940195,
            0.7504883828755602,
            -0.15296486218853164,
            -0.5794684678643381,
            0.3239847771997537,
            0.6740059517812944,
            0.3239847771997537,
            -0.6740059517812944,
            0.5794684678643381,
            -0.3239847771997537,
            -0.3239847771997537,
            -0.7504883828755602,
            0.5029860367700724,
            -0.4004672082940195,
            0.15296486218853164,
            -0.7504883828755602,
            0.15296486218853164,
            -0.4004672082940195,
            0.5029860367700724,
            -0.8828161875373585,
            0.4553054119602712,
            0.08164729285680945,
            0.08164729285680945,
            -0.4553054119602712,
            0.8828161875373585,
            -0.08164729285680945,
            -0.08164729285680945,
            -0.5029860367700724,
            0.7504883828755602,
            -0.15296486218853164,
            0.4004672082940195,
            -0.5029860367700724,
            0.7504883828755602,
            0.4004672082940195,
            -0.15296486218853164,
            -0.5794684678643381,
            0.6740059517812944,
            0.3239847771997537,
            0.3239847771997537,
            0.5794684678643381,
            -0.6740059517812944,
            -0.3239847771997537,
            -0.3239847771997537,
            0.5029860367700724,
            -0.7504883828755602,
            -0.4004672082940195,
            0.15296486218853164,
            0.5029860367700724,
            -0.7504883828755602,
            0.15296486218853164,
            -0.4004672082940195,
            0.4553054119602712,
            -0.8828161875373585,
            0.08164729285680945,
            0.08164729285680945,
            0.8828161875373585,
            -0.4553054119602712,
            -0.08164729285680945,
            -0.08164729285680945,
            0.7504883828755602,
            -0.5029860367700724,
            -0.15296486218853164,
            0.4004672082940195,
            0.7504883828755602,
            -0.5029860367700724,
            0.4004672082940195,
            -0.15296486218853164,
            0.6740059517812944,
            -0.5794684678643381,
            0.3239847771997537,
            0.3239847771997537,
            0.5794684678643381,
            0.6740059517812944,
            -0.3239847771997537,
            -0.3239847771997537,
            0.5029860367700724,
            0.7504883828755602,
            -0.4004672082940195,
            0.15296486218853164,
            0.5029860367700724,
            0.7504883828755602,
            0.15296486218853164,
            -0.4004672082940195,
            0.4553054119602712,
            0.8828161875373585,
            0.08164729285680945,
            -0.08164729285680945,
            0.8828161875373585,
            0.4553054119602712,
            -0.08164729285680945,
            0.08164729285680945,
            0.7504883828755602,
            0.5029860367700724,
            -0.15296486218853164,
            0.4004672082940195,
            0.7504883828755602,
            0.5029860367700724,
            0.4004672082940195,
            -0.15296486218853164,
            0.6740059517812944,
            0.5794684678643381,
            0.3239847771997537,
            -0.3239847771997537,
            // second block from Java (after the separator)
            -0.753341017856078,
            -0.37968289875261624,
            -0.37968289875261624,
            -0.37968289875261624,
            -0.7821684431180708,
            -0.4321472685365301,
            -0.4321472685365301,
            0.12128480194602098,
            -0.7821684431180708,
            -0.4321472685365301,
            0.12128480194602098,
            -0.4321472685365301,
            -0.7821684431180708,
            0.12128480194602098,
            -0.4321472685365301,
            -0.4321472685365301,
            -0.8586508742123365,
            -0.508629699630796,
            0.044802370851755174,
            0.044802370851755174,
            -0.8586508742123365,
            0.044802370851755174,
            -0.508629699630796,
            0.044802370851755174,
            -0.8586508742123365,
            0.044802370851755174,
            0.044802370851755174,
            -0.508629699630796,
            -0.9982828964265062,
            -0.03381941603233842,
            -0.03381941603233842,
            -0.03381941603233842,
            -0.37968289875261624,
            -0.753341017856078,
            -0.37968289875261624,
            -0.37968289875261624,
            -0.4321472685365301,
            -0.7821684431180708,
            -0.4321472685365301,
            0.12128480194602098,
            -0.4321472685365301,
            -0.7821684431180708,
            0.12128480194602098,
            -0.4321472685365301,
            0.12128480194602098,
            -0.7821684431180708,
            -0.4321472685365301,
            -0.4321472685365301,
            -0.508629699630796,
            -0.8586508742123365,
            0.044802370851755174,
            0.044802370851755174,
            0.044802370851755174,
            -0.8586508742123365,
            -0.508629699630796,
            0.044802370851755174,
            0.044802370851755174,
            -0.8586508742123365,
            0.044802370851755174,
            -0.508629699630796,
            -0.03381941603233842,
            -0.9982828964265062,
            -0.03381941603233842,
            -0.03381941603233842,
            -0.37968289875261624,
            -0.37968289875261624,
            -0.753341017856078,
            -0.37968289875261624,
            -0.4321472685365301,
            -0.4321472685365301,
            -0.7821684431180708,
            0.12128480194602098,
            -0.4321472685365301,
            0.12128480194602098,
            -0.7821684431180708,
            -0.4321472685365301,
            -0.508629699630796,
            0.044802370851755174,
            -0.8586508742123365,
            0.044802370851755174,
            0.044802370851755174,
            -0.508629699630796,
            -0.8586508742123365,
            0.044802370851755174,
            0.044802370851755174,
            0.044802370851755174,
            -0.8586508742123365,
            -0.508629699630796,
            -0.03381941603233842,
            -0.03381941603233842,
            -0.9982828964265062,
            -0.03381941603233842,
            -0.37968289875261624,
            -0.37968289875261624,
            -0.37968289875261624,
            -0.753341017856078,
            -0.4321472685365301,
            -0.4321472685365301,
            0.12128480194602098,
            -0.7821684431180708,
            -0.4321472685365301,
            0.12128480194602098,
            -0.4321472685365301,
            -0.7821684431180708,
            -0.508629699630796,
            0.044802370851755174,
            0.044802370851755174,
            -0.8586508742123365,
            0.044802370851755174,
            -0.508629699630796,
            0.044802370851755174,
            -0.8586508742123365,
            0.044802370851755174,
            0.044802370851755174,
            -0.508629699630796,
            -0.8586508742123365,
            -0.03381941603233842,
            -0.03381941603233842,
            -0.03381941603233842,
            -0.9982828964265062,
            // remaining unique quaternions from the Java file (indices 32-63 of block 2)
            -0.3239847771997537,
            -0.6740059517812944,
            -0.3239847771997537,
            0.5794684678643381,
            -0.4004672082940195,
            -0.7504883828755602,
            0.15296486218853164,
            0.5029860367700724,
            0.15296486218853164,
            -0.7504883828755602,
            -0.4004672082940195,
            0.5029860367700724,
            0.08164729285680945,
            -0.8828161875373585,
            0.08164729285680945,
            0.4553054119602712,
            -0.08164729285680945,
            -0.4553054119602712,
            -0.08164729285680945,
            0.8828161875373585,
            -0.15296486218853164,
            -0.5029860367700724,
            0.4004672082940195,
            0.7504883828755602,
            0.4004672082940195,
            -0.5029860367700724,
            -0.15296486218853164,
            0.7504883828755602,
            0.3239847771997537,
            -0.5794684678643381,
            0.3239847771997537,
            0.6740059517812944,
            -0.3239847771997537,
            -0.6740059517812944,
            -0.3239847771997537,
            0.5794684678643381,
            -0.4004672082940195,
            -0.7504883828755602,
            0.15296486218853164,
            0.5029860367700724,
            0.15296486218853164,
            -0.7504883828755602,
            -0.4004672082940195,
            0.5029860367700724,
            0.08164729285680945,
            0.08164729285680945,
            -0.8828161875373585,
            0.4553054119602712,
            -0.08164729285680945,
            -0.08164729285680945,
            -0.4553054119602712,
            0.8828161875373585,
            -0.15296486218853164,
            0.4004672082940195,
            -0.5029860367700724,
            0.7504883828755602,
            0.4004672082940195,
            -0.15296486218853164,
            -0.5029860367700724,
            0.7504883828755602,
            0.3239847771997537,
            0.3239847771997537,
            -0.5794684678643381,
            0.6740059517812944,
            -0.3239847771997537,
            -0.6740059517812944,
            0.5794684678643381,
            -0.3239847771997537,
            -0.4004672082940195,
            -0.7504883828755602,
            0.5029860367700724,
            0.15296486218853164,
            0.15296486218853164,
            -0.7504883828755602,
            0.5029860367700724,
            -0.4004672082940195,
            0.08164729285680945,
            -0.8828161875373585,
            0.4553054119602712,
            0.08164729285680945,
            -0.08164729285680945,
            -0.4553054119602712,
            0.8828161875373585,
            -0.08164729285680945,
            -0.15296486218853164,
            -0.5029860367700724,
            0.7504883828755602,
            0.4004672082940195,
            0.4004672082940195,
            -0.5029860367700724,
            0.7504883828755602,
            -0.15296486218853164,
            0.3239847771997537,
            -0.5794684678643381,
            0.6740059517812944,
            0.3239847771997537,
            -0.3239847771997537,
            0.6740059517812944,
            -0.5794684678643381,
            -0.3239847771997537,
            -0.4004672082940195,
            0.7504883828755602,
            -0.5029860367700724,
            0.15296486218853164,
            0.15296486218853164,
            0.7504883828755602,
            -0.5029860367700724,
            -0.4004672082940195,
            0.08164729285680945,
            0.4553054119602712,
            -0.8828161875373585,
            0.08164729285680945,
            -0.08164729285680945,
            0.8828161875373585,
            -0.4553054119602712,
            -0.08164729285680945,
            -0.15296486218853164,
            0.7504883828755602,
            -0.5029860367700724,
            0.4004672082940195,
            0.4004672082940195,
            0.7504883828755602,
            -0.5029860367700724,
            -0.15296486218853164,
            0.3239847771997537,
            0.6740059517812944,
            -0.5794684678643381,
            -0.3239847771997537,
            // third unique set (0.0338... block from Java)
            0.03381941603233842,
            0.03381941603233842,
            0.03381941603233842,
            0.9982828964265062,
            -0.044802370851755174,
            -0.044802370851755174,
            0.508629699630796,
            0.8586508742123365,
            -0.044802370851755174,
            0.508629699630796,
            -0.044802370851755174,
            0.8586508742123365,
            -0.12128480194602098,
            0.4321472685365301,
            0.4321472685365301,
            0.7821684431180708,
            0.508629699630796,
            -0.044802370851755174,
            -0.044802370851755174,
            0.8586508742123365,
            0.4321472685365301,
            -0.12128480194602098,
            0.4321472685365301,
            0.7821684431180708,
            0.4321472685365301,
            0.4321472685365301,
            -0.12128480194602098,
            0.7821684431180708,
            0.37968289875261624,
            0.37968289875261624,
            0.37968289875261624,
            0.753341017856078,
            0.03381941603233842,
            0.03381941603233842,
            0.9982828964265062,
            0.03381941603233842,
            -0.044802370851755174,
            -0.044802370851755174,
            0.8586508742123365,
            0.508629699630796,
            -0.044802370851755174,
            0.508629699630796,
            0.8586508742123365,
            -0.044802370851755174,
            -0.12128480194602098,
            0.7821684431180708,
            0.4321472685365301,
            0.4321472685365301,
            0.508629699630796,
            -0.044802370851755174,
            0.8586508742123365,
            -0.044802370851755174,
            0.4321472685365301,
            -0.12128480194602098,
            0.7821684431180708,
            0.4321472685365301,
            0.4321472685365301,
            0.4321472685365301,
            -0.12128480194602098,
            0.7821684431180708,
            0.37968289875261624,
            0.37968289875261624,
            0.753341017856078,
            0.37968289875261624,
            0.03381941603233842,
            0.9982828964265062,
            0.03381941603233842,
            0.03381941603233842,
            -0.044802370851755174,
            0.8586508742123365,
            -0.044802370851755174,
            0.508629699630796,
            -0.044802370851755174,
            0.8586508742123365,
            0.508629699630796,
            -0.044802370851755174,
            -0.12128480194602098,
            0.7821684431180708,
            0.4321472685365301,
            0.4321472685365301,
            0.508629699630796,
            -0.044802370851755174,
            0.8586508742123365,
            -0.044802370851755174,
            0.4321472685365301,
            -0.12128480194602098,
            0.7821684431180708,
            0.4321472685365301,
            0.4321472685365301,
            0.4321472685365301,
            -0.12128480194602098,
            0.7821684431180708,
            0.37968289875261624,
            0.37968289875261624,
            0.753341017856078,
            0.37968289875261624,
            0.37968289875261624,
            0.37968289875261624,
            0.37968289875261624,
            0.753341017856078,
            0.4321472685365301,
            0.4321472685365301,
            0.12128480194602098,
            0.7821684431180708,
            0.4321472685365301,
            0.12128480194602098,
            0.4321472685365301,
            0.7821684431180708,
            0.508629699630796,
            0.044802370851755174,
            0.044802370851755174,
            0.8586508742123365,
            0.044802370851755174,
            0.508629699630796,
            0.044802370851755174,
            0.8586508742123365,
            0.044802370851755174,
            0.044802370851755174,
            0.508629699630796,
            0.8586508742123365,
            0.03381941603233842,
            0.03381941603233842,
            0.03381941603233842,
            0.9982828964265062,
            0.8586508742123365,
            -0.044802370851755174,
            -0.044802370851755174,
            0.508629699630796,
            0.8586508742123365,
            -0.044802370851755174,
            0.508629699630796,
            -0.044802370851755174,
            0.7821684431180708,
            -0.12128480194602098,
            0.4321472685365301,
            0.4321472685365301,
            0.8586508742123365,
            0.044802370851755174,
            -0.508629699630796,
            0.044802370851755174,
            0.7821684431180708,
            0.4321472685365301,
            -0.12128480194602098,
            0.4321472685365301,
            0.7821684431180708,
            0.4321472685365301,
            0.4321472685365301,
            -0.12128480194602098,
            0.753341017856078,
            0.37968289875261624,
            0.37968289875261624,
            0.37968289875261624,
        ];
        let mut normalized = raw;
        for value in normalized.iter_mut() {
            *value = (*value as f64 / NORMALIZER_4D) as f32;
        }
        let mut out = vec![0.0_f32; N_GRADS_4D as usize * 4];
        let mut j = 0;
        for slot in out.iter_mut() {
            if j == normalized.len() {
                j = 0;
            }
            *slot = normalized[j];
            j += 1;
        }
        out
    })
}

#[cfg(test)]
mod tests {
    use super::Simplex;

    #[test]
    fn noise2_is_deterministic_for_same_seed_and_coords() {
        let a = Simplex::new(12_345);
        let b = Simplex::new(12_345);
        for &(x, y) in &[
            (0.0_f64, 0.0),
            (1.25, -3.5),
            (-10.0, 20.0),
            (0.5, 0.5),
            (100.25, -50.75),
        ] {
            assert_eq!(a.noise2(x, y), b.noise2(x, y));
            assert_eq!(a.noise2_improve_x(x, y), b.noise2_improve_x(x, y));
        }
    }

    #[test]
    fn noise2_differs_across_seeds() {
        let a = Simplex::new(1);
        let b = Simplex::new(2);
        assert_ne!(a.noise2(1.0, 2.0), b.noise2(1.0, 2.0));
    }

    #[test]
    fn noise2_values_stay_in_unit_range() {
        let noise = Simplex::new(99);
        for i in 0..64 {
            let x = i as f64 * 0.37;
            let y = i as f64 * -0.19;
            let value = noise.noise2(x, y);
            assert!(
                (-1.0..=1.0).contains(&value),
                "noise2 out of range at ({x},{y}): {value}"
            );
        }
    }

    #[test]
    fn noise3_restores_seed_so_repeats_match() {
        let mut noise = Simplex::new(7_777);
        let first = noise.noise3(1.5, -2.25, 3.0);
        let second = noise.noise3(1.5, -2.25, 3.0);
        assert_eq!(first, second);
        assert_eq!(noise.seed(), 7_777);
    }

    #[test]
    fn noise3_is_deterministic_for_same_seed() {
        let mut a = Simplex::new(42);
        let mut b = Simplex::new(42);
        for i in 0..32 {
            let x = i as f64 * 0.11;
            let y = i as f64 * 0.22;
            let z = i as f64 * 0.33;
            assert_eq!(a.noise3(x, y, z), b.noise3(x, y, z));
            assert_eq!(a.noise3_improve_xz(x, y, z), b.noise3_improve_xz(x, y, z));
        }
    }

    #[test]
    fn noise4_restores_seed() {
        let mut noise = Simplex::new(5);
        let first = noise.noise4_fallback(0.5, 1.5, 2.5, 3.5);
        let second = noise.noise4_fallback(0.5, 1.5, 2.5, 3.5);
        assert_eq!(first, second);
        assert_eq!(noise.seed(), 5);
    }
}
