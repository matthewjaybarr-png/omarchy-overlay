//! The month grid behind the bar clock's calendar, as Omarchy's clock opens one: pure date
//! arithmetic, so it is tested on any platform.

/// Days in `month` (1–12) of `year`, in the Gregorian calendar.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 31,
    }
}

/// Day of the week of a date: 0 is Monday, 6 is Sunday.
pub fn weekday(year: i32, month: u32, day: u32) -> u32 {
    // Sakamoto's method, which counts from Sunday.
    const OFFSETS: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    let sunday_first = (y + y / 4 - y / 100 + y / 400 + OFFSETS[(month - 1) as usize] + day as i32).rem_euclid(7);
    ((sunday_first + 6) % 7) as u32
}

/// The month `delta` months away from `month` of `year`.
pub fn step(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let index = year * 12 + month as i32 - 1 + delta;
    (index.div_euclid(12), (index.rem_euclid(12) + 1) as u32)
}

/// The weeks of a month as rows of seven cells, starting on `first_day` (0 Monday …
/// 6 Sunday, as Windows' LOCALE_IFIRSTDAYOFWEEK gives it). Cells outside the month are
/// None. Always six rows, so the panel keeps one size from month to month.
pub fn grid(year: i32, month: u32, first_day: u32) -> [[Option<u32>; 7]; 6] {
    let mut rows = [[None; 7]; 6];
    let lead = (weekday(year, month, 1) + 7 - first_day % 7) % 7;
    for day in 1..=days_in_month(year, month) {
        let cell = (lead + day - 1) as usize;
        rows[cell / 7][cell % 7] = Some(day);
    }
    rows
}

/// The column labels' order: indexes into Monday-first day names, starting at `first_day`.
pub fn weekday_order(first_day: u32) -> [usize; 7] {
    std::array::from_fn(|i| (first_day as usize + i) % 7)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_lengths_follow_leap_years() {
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn weekdays_match_known_dates() {
        assert_eq!(weekday(2026, 11, 26), 3); // a Thursday
        assert_eq!(weekday(2000, 1, 1), 5); // a Saturday
        assert_eq!(weekday(2024, 2, 29), 3); // a Thursday
        assert_eq!(weekday(2026, 3, 1), 6); // a Sunday
    }

    #[test]
    fn stepping_crosses_years() {
        assert_eq!(step(2026, 12, 1), (2027, 1));
        assert_eq!(step(2026, 1, -1), (2025, 12));
        assert_eq!(step(2026, 11, -23), (2024, 12));
        assert_eq!(step(2026, 11, 0), (2026, 11));
    }

    #[test]
    fn grid_places_the_first_on_its_weekday() {
        // November 2026 starts on a Sunday.
        let monday_first = grid(2026, 11, 0);
        assert_eq!(monday_first[0][6], Some(1));
        assert_eq!(monday_first[0][5], None);
        let sunday_first = grid(2026, 11, 6);
        assert_eq!(sunday_first[0][0], Some(1));
        let days: Vec<u32> = sunday_first.iter().flatten().flatten().copied().collect();
        assert_eq!(days, (1..=30).collect::<Vec<_>>());
    }

    #[test]
    fn every_month_fits_six_rows() {
        for year in [2026, 2027, 2028] {
            for month in 1..=12 {
                for first in 0..7 {
                    let count = grid(year, month, first).iter().flatten().flatten().count() as u32;
                    assert_eq!(count, days_in_month(year, month));
                }
            }
        }
    }

    #[test]
    fn weekday_labels_start_on_the_first_day() {
        assert_eq!(weekday_order(0), [0, 1, 2, 3, 4, 5, 6]);
        assert_eq!(weekday_order(6), [6, 0, 1, 2, 3, 4, 5]);
    }
}
