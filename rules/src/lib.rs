use std::{hash::Hash, time::{Duration, SystemTime}};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rules<Player> {
    Grief,
    Waste,
    Nsfw,
    Bypass(Player),
}

impl<Player> Rules<Player> {
    pub const fn info(&self) -> RuleInfo {
        use Rules::*;
        match *self {
            Grief => RuleInfo {
                tag: "grief",
                desc_en: "Intentionally causing harm to your team.",
                desc_ru: "Умышленное причинение вреда своей команде.",
                duration: days(7)
            },
            Waste => RuleInfo {
                tag: "waste",
                desc_en: "A waste of resources or free space.",
                desc_ru: "Бесполезная трата ресурсов или свободного места.",
                duration: days(1)
            },
            Nsfw => RuleInfo {
                tag: "nsfw",
                desc_en: "test",
                desc_ru: "test",
                duration: days(1)
            },
            Bypass{..} => RuleInfo {
                tag: "bypass",
                desc_en: "test",
                desc_ru: "test",
                duration: days(7)
            },
        }
    }
}

pub struct RuleInfo {
    pub tag: &'static str,
    pub desc_en: &'static str,
    pub desc_ru: &'static str,
    pub duration: Duration,
}

const fn days(num: u64) -> Duration {
    Duration::from_secs(num * 24 * 60 * 60)
}

pub fn get_remaining_ban_time(warns: impl Iterator<Item = (Rules<impl Eq + Hash + Clone>, SystemTime)>) -> Option<Duration> {
    // \sum\limits_{i=0}^N\Big(T_i*max(0,1-\frac{dt_{i,0}}{720})+(e-1)\sum\limits_{j=1}^KT_ie^{j-1}max(0,1-\frac{dt_{i,j}}{720})\Big)
    // TODO: split_last_mut
    let mut warns = warns.peekable();
    warns
        .peek()
        .cloned()
        .map(|the_last| warns
            .into_group_map()
            .into_iter()
            .map(|(rule, issued)| issued
                .into_iter()
                .map(|issued| rule
                    .info().duration.as_secs_f64() 
                    * 0f64.max(
                        1. - the_last.1.duration_since(issued)
                            .expect("ban from the future?")
                            .as_secs_f64() / days(720).as_secs_f64()
                    )
                )
                .batching(|it| it
                    .next()
                    .map(|last| last
                        + (std::f64::consts::E - 1.)
                        * (it
                            .enumerate()
                            .map(|(i, d)|
                                d * (i as f64).exp()
                            )
                            .sum::<f64>()
                    )
                    )
                )
                .next()
                .unwrap_or_default()
            )
            .sum::<f64>() - (
            SystemTime::now().duration_since(the_last.1)
                .expect("ban from the future?")
                .as_secs_f64()
        )
        )
        .filter(|d| *d > 0.0)
        .map(|secs|
            Duration::from_secs_f64(secs)
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    type Rules = super::Rules<()>;

    #[test]
    fn get_remaining_ban_time_test_empty() {
        let warns: [(Rules, SystemTime); 0] = [];
        assert!(get_remaining_ban_time(warns.into_iter()) == None);
    }

    #[test]
    fn get_remaining_ban_time_test_expired() {
        let warns = [
            (Rules::Waste, SystemTime::now() - days(1)),
        ];
        assert!(get_remaining_ban_time(warns.into_iter()) == None);
    }

    #[test]
    fn get_remaining_ban_time_test_long() {
        let warns = [
            (Rules::Grief, SystemTime::now() - days(0)); 10
        ];
        assert!((-get_remaining_ban_time(warns.into_iter()).unwrap().as_secs_f64()
            + Rules::Grief.info().duration.as_secs_f64() * 9f64.exp()
        ).abs() < 1.0);
    }

    #[test]
    fn get_remaining_ban_time_test_shift() {
        let warns = [
            (Rules::Grief, SystemTime::now() - days(5)),
        ];
        assert!((-get_remaining_ban_time(warns.into_iter()).unwrap().as_secs_f64()
            + Rules::Grief.info().duration.as_secs_f64()
            - days(5).as_secs_f64()
        ).abs() < 1.0);
    }

    #[test]
    fn get_remaining_ban_time_test_complex() {
        let warns = [
            (Rules::Grief, SystemTime::now() - days(5)),
            (Rules::Grief, SystemTime::now() - days(360+5)),
            (Rules::Waste, SystemTime::now() - days(360+5)),
            (Rules::Waste, SystemTime::now() - days(720+5)),
        ];
        assert!((-get_remaining_ban_time(warns.into_iter()).unwrap().as_secs_f64()
            + Rules::Grief.info().duration.as_secs_f64() + (std::f64::consts::E - 1.) * Rules::Grief.info().duration.as_secs_f64() * 0.5f64
            + Rules::Waste.info().duration.as_secs_f64() * 0.5f64
            - days(5).as_secs_f64()

        ).abs() < 1.0);
    }
}
