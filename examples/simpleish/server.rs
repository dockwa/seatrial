use chrono::prelude::*;
use vial::prelude::*;

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::Duration;

routes! {
    GET "/calendar" => calendar;
}

#[derive(Debug)]
enum CalendarError {
    UnparseableDates {
        source: CalendarErrorSource,
        query_param: &'static str,
    },
}

#[derive(Debug)]
enum CalendarErrorSource {
    Chrono(chrono::ParseError),
    Vial,
}

impl Display for CalendarError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Calendar Error: {}",
            match self {
                CalendarError::UnparseableDates {
                    query_param,
                    source,
                } => format!(
                    "query param unparseable as date: {} ({:?})",
                    query_param, source
                ),
            }
        )
    }
}

impl Error for CalendarError {}

fn calendar(req: Request) -> Result<String, CalendarError> {
    let start_date: NaiveDate = req
        .query("start_date")
        .ok_or(())
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
        .map_err(|_| CalendarError::UnparseableDates {
            source: CalendarErrorSource::Vial,
            query_param: "start_date",
        })?
        .map_err(|err| CalendarError::UnparseableDates {
            source: CalendarErrorSource::Chrono(err),
            query_param: "start_date",
        })?;
    let end_date: NaiveDate = req
        .query("end_date")
        .ok_or(())
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
        .map_err(|_| CalendarError::UnparseableDates {
            source: CalendarErrorSource::Vial,
            query_param: "end_date",
        })?
        .map_err(|err| CalendarError::UnparseableDates {
            source: CalendarErrorSource::Chrono(err),
            query_param: "end_date",
        })?;

    let diff = end_date - start_date;

    // sleep for up to two seconds to simulate doing work
    let artificial_delay = Duration::from_millis((rand::random::<f64>() * 2000.00).round() as u64);
    std::thread::sleep(artificial_delay);

    // some contrived esoteric format just designed to give the LuaFunction validator something
    // worth doing
    let response = format!(
        "DAYS {} SYEAR {} EYEAR {} SMON {} EMON {} SDAY {} EDAY {}",
        diff.num_days(),
        start_date.year(),
        end_date.year(),
        start_date.month(),
        end_date.month(),
        start_date.day(),
        end_date.day(),
    );

    eprintln!("debug: {}", response);

    Ok(response)
}

fn main() {
    vial::run!().unwrap();
}
