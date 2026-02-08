use std::convert::TryFrom;

use jiff::Timestamp;

use super::FileTime;

impl TryFrom<FileTime> for Timestamp {
    type Error = jiff::Error;

    fn try_from(value: FileTime) -> Result<Self, Self::Error> {
        Timestamp::new(value.unix_seconds(), value.nanoseconds().cast_signed())
    }
}

impl From<Timestamp> for FileTime {
    fn from(value: Timestamp) -> Self {
        FileTime::from_unix_time(value.as_second(), value.subsec_nanosecond().cast_unsigned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filetime_to_from_jiff() {
        let file_time = FileTime::from_unix_time(1234, 56789);
        let jiff_time = Timestamp::try_from(file_time).expect("Conversion should succeed");
        assert_eq!(jiff_time.as_second(), 1234);
        assert_eq!(jiff_time.subsec_nanosecond(), 56789);
        assert_eq!(jiff_time.as_nanosecond(), 1234 * 1_000_000_000 + 56789);

        let file_time_converted_back = FileTime::from(jiff_time);
        assert_eq!(file_time_converted_back.unix_seconds(), 1234);
        assert_eq!(file_time_converted_back.nanoseconds(), 56789);
        assert_eq!(file_time_converted_back, file_time);
    }
}
