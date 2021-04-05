pub const BANNER: &str = r#"

 _____  _____       _____  _____  _____  __ __  _____  _____ 
 /  _  \/  _  \ ___ /  ___>/   __\/  _  \/  |  \/   __\/  _  \
 |   __/|   __/<___>|___  ||   __||  _  <\  |  /|   __||  _  <
 \__/   \__/        <_____/\_____/\__|\_/ \___/ \_____/\__|\_/
                                                              
 
"#;

#[cfg(feature = "peace")]
pub const DB_VERSION: &str = "0.8.5";
#[cfg(feature = "peace")]
pub const PEACE_VERSION: &str = "0.6.4";

#[derive(Debug, Clone)]
pub enum RankStatusInServer {
    NotSubmitted    = -1,
    Pending         = 0,
    Outdated        = 1,
    Ranked          = 2,
    Approved        = 3,
    Qualified       = 4,
    Loved           = 5,
    Unknown,
}

impl RankStatusInServer {
    #[inline(always)]
    pub fn from_api_rank_status(i: i32) -> Self {
        match i {
            -2 => Self::Pending,
            -1 => Self::Pending,
            0 => Self::Pending,
            1 => Self::Ranked,
            2 => Self::Approved,
            3 => Self::Qualified,
            4 => Self::Loved,
            _ => Self::Unknown,
        }
    }
}
