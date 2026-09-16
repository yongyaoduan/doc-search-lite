use anyhow::{Context, Result, bail, ensure};
use calamine::{Reader as _, Xlsx, open_workbook};
use quick_xml::{Reader, events::Event};
use std::{fs::File, io::Read, path::Path};

// Locations survive indexing; text conversion never needs a language model.
pub struct Section {
    pub location: String,
    pub text: String,
}

pub fn supported(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "txt" | "md" | "csv" | "pdf" | "docx" | "pptx" | "xlsx"
    )
}

fn extension(path: &Path) -> String {
    path.extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}

pub fn extract(path: &Path) -> Result<Vec<Section>> {
    ensure!(
        path.metadata()?.len() <= 100 * 1024 * 1024,
        "file exceeds 100 MiB limit"
    );
    let sections = match extension(path).as_str() {
        "txt" | "md" | "csv" => vec![Section {
            location: "text".into(),
            text: std::fs::read_to_string(path).context("text files must be UTF-8")?,
        }],
        "pdf" => {
            let doc = pdf_extract::Document::load(path)?;
            ensure!(
                !doc.is_encrypted(),
                "password-protected PDFs are not supported"
            );
            let mut sections = Vec::new();
            for page in doc.get_pages().keys() {
                let mut text = String::new();
                // The layout extractor supports form XObjects but not all named CJK
                // encodings. Its underlying PDF parser can decode those encodings.
                let extracted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    pdf_extract::output_doc_page(
                        &doc,
                        &mut pdf_extract::PlainTextOutput::new(&mut text),
                        *page,
                    )
                }));
                if !matches!(extracted, Ok(Ok(()))) {
                    text = doc
                        .extract_text(&[*page])
                        .with_context(|| format!("cannot extract PDF page {page}"))?;
                }
                sections.push(Section {
                    location: format!("page {page}"),
                    text,
                });
            }
            sections
        }
        "docx" | "pptx" => office(path)?,
        "xlsx" => {
            let mut book: Xlsx<_> = open_workbook(path)?;
            let mut sections = Vec::new();
            for name in book.sheet_names().to_vec() {
                let range = book.worksheet_range(&name)?;
                let first_row = range.start().map_or(0, |(row, _)| row);
                let header = range
                    .rows()
                    .next()
                    .map(|row| {
                        row.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join("\t")
                    })
                    .unwrap_or_default();
                for (i, row) in range.rows().enumerate() {
                    let text = row
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("\t");
                    if !text.trim().is_empty() {
                        sections.push(Section {
                            location: format!("sheet {name}, row {}", first_row as usize + i + 1),
                            text: if i == 0 {
                                text
                            } else {
                                format!("{header}\n{text}")
                            },
                        });
                    }
                }
            }
            sections
        }
        _ => bail!("unsupported file type"),
    };
    ensure!(
        sections.iter().any(|s| !s.text.trim().is_empty()),
        "no extractable text (scanned PDFs need OCR, which is not included)"
    );
    ensure!(
        sections.iter().map(|s| s.text.len()).sum::<usize>() <= 64 * 1024 * 1024,
        "extracted text exceeds 64 MiB limit"
    );
    Ok(sections)
}

fn office(path: &Path) -> Result<Vec<Section>> {
    let mut archive = zip::ZipArchive::new(File::open(path)?)?;
    let mut names: Vec<(usize, String)> = if extension(path) == "docx" {
        vec![(0, "word/document.xml".into())]
    } else {
        let presentation = read_xml(&mut archive, "ppt/presentation.xml")?;
        let relationships = read_xml(&mut archive, "ppt/_rels/presentation.xml.rels")?;
        let mut targets = std::collections::HashMap::new();
        let mut reader = Reader::from_str(&relationships);
        loop {
            match reader.read_event()? {
                Event::Empty(e) | Event::Start(e) if e.local_name().as_ref() == b"Relationship" => {
                    let mut id = String::new();
                    let mut target = String::new();
                    for a in e.attributes() {
                        let a = a?;
                        let value = a
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )?
                            .into_owned();
                        match a.key.as_ref() {
                            b"Id" => id = value,
                            b"Target" => target = value,
                            _ => {}
                        }
                    }
                    targets.insert(id, target);
                }
                Event::Eof => break,
                _ => {}
            }
        }
        let mut reader = Reader::from_str(&presentation);
        let mut slides = Vec::new();
        loop {
            match reader.read_event()? {
                Event::Empty(e) | Event::Start(e) if e.local_name().as_ref() == b"sldId" => {
                    for a in e.attributes() {
                        let a = a?;
                        if a.key.as_ref().ends_with(b":id") {
                            let id = a.decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )?;
                            let target = targets
                                .get(id.as_ref())
                                .context("missing slide relationship")?;
                            let name = if target.starts_with('/') {
                                target.trim_start_matches('/').to_string()
                            } else {
                                format!("ppt/{target}")
                            };
                            slides.push((slides.len() + 1, name));
                        }
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
        slides
    };
    names.sort_by_key(|(number, _)| *number);
    let mut sections = Vec::new();
    let mut total = 0;
    for (number, name) in names {
        let entry = archive.by_name(&name)?;
        let mut xml = String::new();
        entry.take(16 * 1024 * 1024 + 1).read_to_string(&mut xml)?;
        total += xml.len();
        ensure!(
            xml.len() <= 16 * 1024 * 1024 && total <= 64 * 1024 * 1024,
            "Office XML exceeds size limit"
        );
        sections.push(Section {
            location: if number == 0 {
                "document".into()
            } else {
                format!("slide {number}")
            },
            text: xml_text(&xml)?,
        });
    }
    Ok(sections)
}

fn read_xml(archive: &mut zip::ZipArchive<File>, name: &str) -> Result<String> {
    let mut xml = String::new();
    archive
        .by_name(name)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_string(&mut xml)?;
    ensure!(
        xml.len() <= 16 * 1024 * 1024,
        "Office XML exceeds size limit"
    );
    Ok(xml)
}

fn xml_text(xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml);
    let mut text = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.local_name().as_ref() == b"t" => in_text = true,
            Event::End(e) => match e.local_name().as_ref() {
                b"t" => in_text = false,
                b"p" | b"tr" => text.push('\n'),
                b"tc" => text.push('\t'),
                _ => {}
            },
            Event::Empty(e) => match e.local_name().as_ref() {
                b"br" | b"cr" => text.push('\n'),
                b"tab" => text.push('\t'),
                _ => {}
            },
            Event::Text(e) if in_text => text.push_str(&e.decode()?),
            Event::CData(e) if in_text => text.push_str(&e.decode()?),
            Event::GeneralRef(e) if in_text => {
                let entity = format!("&{};", e.decode()?);
                text.push_str(&quick_xml::escape::unescape(&entity)?);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(text)
}
