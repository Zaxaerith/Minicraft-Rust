//! Java `minicraft.saveload.Version` — parse and compare dotted save versions.
//!
//! Layout mirrors `Version.java` 1:1: `make.major.minor[-pre|dev|infdev N]`.

use std::cmp::Ordering;

/// Java `Version`. `dev == 0` means a release build of that triple.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    make: i32,
    major: i32,
    minor: i32,
    dev: i32,
    valid: bool,
    is_special: bool,
}

impl Version {
    /// Java `new Version(version)` — logs invalid numbers in the Java client;
    /// here invalid input is kept as `valid == false`.
    pub fn parse(version: &str) -> Self {
        Self::parse_inner(version)
    }

    /// Java `Version.isValid(String)`.
    pub fn is_valid_string(version: &str) -> bool {
        Self::parse_inner(version).is_valid()
    }

    fn parse_inner(version: &str) -> Self {
        let mut parsed = Self {
            make: 0,
            major: 0,
            minor: 0,
            dev: 0,
            valid: true,
            is_special: false,
        };
        let nums: Vec<&str> = version.split('.').collect();
        match nums[0].parse::<i32>() {
            Ok(value) => parsed.make = value,
            Err(_) => {
                parsed.valid = false;
                return parsed;
            }
        }
        if nums.len() > 1 {
            match nums[1].parse::<i32>() {
                Ok(value) => parsed.major = value,
                Err(_) => {
                    parsed.valid = false;
                    return parsed;
                }
            }
        }
        let min = if nums.len() > 2 { nums[2] } else { "" };
        if min.contains('-') {
            let mindev: Vec<&str> = min.split('-').collect();
            match mindev[0].parse::<i32>() {
                Ok(value) => parsed.minor = value,
                Err(_) => {
                    parsed.valid = false;
                    return parsed;
                }
            }
            parsed.is_special = mindev[1].contains("infdev");
            let cleaned = if parsed.is_special {
                mindev[1].replace("pre", "").replace("infdev", "")
            } else {
                mindev[1].replace("pre", "").replace("dev", "")
            };
            match cleaned.parse::<i32>() {
                Ok(value) => parsed.dev = value,
                Err(_) => {
                    parsed.valid = false;
                }
            }
        } else if !min.is_empty() {
            match min.parse::<i32>() {
                Ok(value) => parsed.minor = value,
                Err(_) => parsed.valid = false,
            }
        }
        parsed
    }

    pub fn is_special(&self) -> bool {
        self.is_special
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Java `compareTo(Version)`.
    pub fn compare(&self, other: &Self) -> Ordering {
        self.compare_ignore_dev(other, false)
    }

    /// Java `compareTo(Version, boolean ignoreDev)`.
    pub fn compare_ignore_dev(&self, other: &Self, ignore_dev: bool) -> Ordering {
        if self.make != other.make {
            return self.make.cmp(&other.make);
        }
        if self.major != other.major {
            return self.major.cmp(&other.major);
        }
        if self.minor != other.minor {
            return self.minor.cmp(&other.minor);
        }
        if !ignore_dev && self.dev != other.dev {
            // 0 is the last "dev" version, as it is not a dev.
            if self.dev == 0 {
                return Ordering::Greater;
            }
            if other.dev == 0 {
                return Ordering::Less;
            }
            return self.dev.cmp(&other.dev);
        }
        Ordering::Equal
    }

    pub fn at_least(&self, other: &Self) -> bool {
        self.compare(other) != Ordering::Less
    }

    pub fn less_than(&self, other: &Self) -> bool {
        self.compare(other) == Ordering::Less
    }

    pub fn is_dev(&self) -> bool {
        self.dev != 0
    }

    /// Java `toArray()` → `[make, major, minor, dev]`.
    pub fn to_array(&self) -> [i32; 4] {
        [self.make, self.major, self.minor, self.dev]
    }

    pub fn make(&self) -> i32 {
        self.make
    }

    pub fn major(&self) -> i32 {
        self.major
    }

    pub fn minor(&self) -> i32 {
        self.minor
    }

    pub fn dev(&self) -> i32 {
        self.dev
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Java uses curIsSpecial for the suffix form; the default path prints
        // `-dev N` for non-special builds and omits a zero dev.
        write!(formatter, "{}.{}.{}", self.make, self.major, self.minor)?;
        if self.dev != 0 {
            if self.is_special {
                write!(formatter, "-infdev{}", self.dev)?;
            } else {
                write!(formatter, "-dev{}", self.dev)?;
            }
        }
        Ok(())
    }
}

/// Convenience constructors for the milestones Java `loadGame` branches on.
pub mod milestones {
    use super::Version;

    pub fn v(major: i32, minor: i32, patch: i32) -> Version {
        Version::parse(&format!("{major}.{minor}.{patch}"))
    }

    pub fn v_dev(major: i32, minor: i32, patch: i32, dev: i32) -> Version {
        Version::parse(&format!("{major}.{minor}.{patch}-dev{dev}"))
    }

    pub fn v_1_9_3_dev2() -> Version {
        v_dev(1, 9, 3, 2)
    }

    pub fn v_1_9_3_dev3() -> Version {
        v_dev(1, 9, 3, 3)
    }

    pub fn v_1_9_4_dev4() -> Version {
        v_dev(1, 9, 4, 4)
    }

    pub fn v_2_0_3() -> Version {
        v(2, 0, 3)
    }

    pub fn v_2_0_4_dev7() -> Version {
        v_dev(2, 0, 4, 7)
    }

    pub fn v_2_0_4_dev8() -> Version {
        v_dev(2, 0, 4, 8)
    }

    pub fn v_2_0_6_dev3() -> Version {
        v_dev(2, 0, 6, 3)
    }

    pub fn v_2_0_6_dev4() -> Version {
        v_dev(2, 0, 6, 4)
    }

    pub fn v_2_0_7_dev1() -> Version {
        v_dev(2, 0, 7, 1)
    }

    pub fn v_2_0_7_dev2() -> Version {
        v_dev(2, 0, 7, 2)
    }

    pub fn v_2_1_0() -> Version {
        v(2, 1, 0)
    }

    pub fn v_2_1_0_dev2() -> Version {
        v_dev(2, 1, 0, 2)
    }

    pub fn v_2_2_0_dev1() -> Version {
        v_dev(2, 2, 0, 1)
    }

    pub fn v_2_2_0_dev3() -> Version {
        v_dev(2, 2, 0, 3)
    }

    pub fn v_2_2_0_dev4() -> Version {
        v_dev(2, 2, 0, 4)
    }

    pub fn v_2_2_0_dev6() -> Version {
        v_dev(2, 2, 0, 6)
    }

    pub fn v_2_2_4() -> Version {
        v(2, 2, 4)
    }
}

#[cfg(test)]
mod tests {
    use super::{Version, milestones};
    use std::cmp::Ordering;

    #[test]
    fn parses_release_and_dev_triples() {
        let release = Version::parse("2.2.4");
        assert!(release.is_valid());
        assert!(!release.is_dev());
        assert_eq!(release.to_array(), [2, 2, 4, 0]);
        assert_eq!(release.to_string(), "2.2.4");

        let dev = Version::parse("2.0.4-dev8");
        assert!(dev.is_valid());
        assert!(dev.is_dev());
        assert_eq!(dev.to_array(), [2, 0, 4, 8]);

        let short = Version::parse("1.8");
        assert_eq!(short.to_array(), [1, 8, 0, 0]);
    }

    #[test]
    fn compares_like_java_including_zero_dev_being_latest() {
        let release = Version::parse("2.0.4");
        let dev8 = Version::parse("2.0.4-dev8");
        assert_eq!(release.compare(&dev8), Ordering::Greater);
        assert_eq!(dev8.compare(&release), Ordering::Less);
        assert_eq!(release.compare(&release), Ordering::Equal);
        assert_eq!(
            Version::parse("2.0.3").compare(&Version::parse("2.0.4-dev1")),
            Ordering::Less
        );
        assert!(milestones::v_2_2_0_dev1().at_least(&milestones::v_2_2_0_dev1()));
        assert!(milestones::v_2_2_4().at_least(&milestones::v_2_2_0_dev3()));
        assert!(milestones::v_2_0_3().less_than(&milestones::v_2_0_4_dev8()));
    }

    #[test]
    fn invalid_versions_are_flagged() {
        assert!(!Version::parse("not-a-version").is_valid());
        assert!(!Version::is_valid_string(""));
        assert!(Version::is_valid_string("2.2.4"));
    }
}
