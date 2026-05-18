//! 결과를 외부 형식(.xlsx)으로 내보내기.
//!
//! 프론트엔드는 현재 표시 중인 `QueryResult` 를 그대로 다시 전송하고,
//! Rust 가 타입을 보존해 셀에 쓴다(숫자/불리언/날짜는 native, 그 외는 문자열).

use crate::error::{AppError, AppResult};
use crate::query::{QueryResult, RowValue};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use rust_xlsxwriter::{ExcelDateTime, Format, Workbook};

#[tauri::command]
pub fn export_query_xlsx(path: String, result: QueryResult) -> AppResult<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // 헤더는 굵게 + 가벼운 배경
    let header_format = Format::new()
        .set_bold()
        .set_background_color(rust_xlsxwriter::Color::RGB(0xEEF2F7));
    let date_format = Format::new().set_num_format("yyyy-mm-dd hh:mm:ss");

    // 헤더
    for (col_idx, col) in result.columns.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col_idx as u16, &col.name, &header_format)
            .map_err(xlsx_err)?;
    }

    // 데이터
    for (row_idx, row) in result.rows.iter().enumerate() {
        let r = (row_idx + 1) as u32;
        for (col_idx, val) in row.iter().enumerate() {
            let c = col_idx as u16;
            write_cell(worksheet, r, c, val, &date_format)?;
        }
    }

    // 헤더 폭 자동 조정 — 데이터까지 보고 결정
    worksheet.autofit();

    workbook.save(&path).map_err(xlsx_err)?;
    Ok(())
}

fn xlsx_err(e: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::Internal(format!("xlsx: {e}"))
}

fn write_cell(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    val: &RowValue,
    date_format: &Format,
) -> AppResult<()> {
    match val {
        RowValue::Null => Ok(()), // 빈 셀
        RowValue::Bool(b) => {
            sheet.write_boolean(row, col, *b).map_err(xlsx_err)?;
            Ok(())
        }
        RowValue::Int(i) => {
            // 안전한 정수 범위 내라면 number 로
            sheet.write_number(row, col, *i as f64).map_err(xlsx_err)?;
            Ok(())
        }
        RowValue::Float(f) => {
            sheet.write_number(row, col, *f).map_err(xlsx_err)?;
            Ok(())
        }
        RowValue::Decimal(s) => {
            // 가능하면 number 로, 안되면 문자열
            if let Ok(n) = s.parse::<f64>() {
                sheet.write_number(row, col, n).map_err(xlsx_err)?;
            } else {
                sheet.write_string(row, col, s).map_err(xlsx_err)?;
            }
            Ok(())
        }
        RowValue::Text(s) => {
            sheet.write_string(row, col, s).map_err(xlsx_err)?;
            Ok(())
        }
        RowValue::DateTime(s) => {
            if let Some(xlsx_dt) = parse_excel_datetime(s) {
                sheet
                    .write_datetime_with_format(row, col, &xlsx_dt, date_format)
                    .map_err(xlsx_err)?;
            } else {
                sheet.write_string(row, col, s).map_err(xlsx_err)?;
            }
            Ok(())
        }
        RowValue::Uuid(s) | RowValue::Binary(s) | RowValue::Unknown(s) => {
            sheet.write_string(row, col, s).map_err(xlsx_err)?;
            Ok(())
        }
    }
}

/// ISO 8601 형태의 문자열을 Excel 의 datetime 값으로 변환.
/// 실패 시 None — 호출자는 문자열로 fallback.
fn parse_excel_datetime(s: &str) -> Option<ExcelDateTime> {
    // 시도 1: 풀 datetime
    if let Ok(dt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
    {
        let date = dt.date();
        let ms = dt.and_utc().timestamp_subsec_millis() as u16;
        return ExcelDateTime::from_ymd(
            date.year() as u16,
            date.month() as u8,
            date.day() as u8,
        )
        .ok()
        .and_then(|d| {
            d.and_hms_milli(
                dt.hour() as u16,
                dt.minute() as u8,
                dt.second() as u8,
                ms,
            )
            .ok()
        });
    }
    // 시도 2: 날짜만
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return ExcelDateTime::from_ymd(d.year() as u16, d.month() as u8, d.day() as u8).ok();
    }
    // 시도 3: 시간만 — Excel datetime 으로는 부적합 → None
    if NaiveTime::parse_from_str(s, "%H:%M:%S%.f").is_ok()
        || NaiveTime::parse_from_str(s, "%H:%M:%S").is_ok()
    {
        return None;
    }
    None
}
