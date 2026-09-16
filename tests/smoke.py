"""Exercise the shipped binary with real document containers; stdlib only.

Python is a build/CI test driver, never part of the installed application.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import zipfile


def office(path, entries):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, text in entries.items():
            archive.writestr(name, text)


def fixtures(folder):
    office(folder / "合同 Word.docx", {
        "[Content_Types].xml": '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
        "_rels/.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
        "word/document.xml": '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>海底</w:t></w:r><w:r><w:t>光缆验收标准 submarine cable acceptance</w:t></w:r></w:p><w:tbl><w:tr><w:tc><w:p><w:r><w:t>合同编号 HN-2026 &amp; 合作方</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>',
    })
    office(folder / "方案 PPT.pptx", {
        "[Content_Types].xml": '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>',
        "ppt/presentation.xml": '<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rId10"/><p:sldId id="257" r:id="rId2"/></p:sldIdLst></p:presentation>',
        "ppt/_rels/presentation.xml.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId10" Target="slides/slide10.xml"/><Relationship Id="rId2" Target="slides/slide2.xml"/></Relationships>',
        "ppt/slides/slide10.xml": '<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>网络建设 rollout schedule</a:t></a:r></a:p></p:sld>',
        "ppt/slides/slide2.xml": '<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:p><a:r><a:t>后续计划 second presentation slide</a:t></a:r></a:p></p:sld>',
    })
    office(folder / "预算 Excel.xlsx", {
        "_rels/.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>',
        "[Content_Types].xml": '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/></Types>',
        "xl/workbook.xml": '<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="项目预算" sheetId="1" r:id="rId1"/></sheets></workbook>',
        "xl/_rels/workbook.xml.rels": '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>',
        "xl/sharedStrings.xml": '<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><si><t>设备采购 equipment procurement</t></si></sst>',
        "xl/worksheets/sheet1.xml": '<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="3"><c r="A3" t="inlineStr"><is><t>项目 budget</t></is></c><c r="B3" t="inlineStr"><is><t>金额</t></is></c></row><row r="4"><c r="A4" t="s"><v>0</v></c><c r="B4"><v>125000</v></c><c r="C4"><f>B4*2</f><v>250000</v></c></row></sheetData></worksheet>',
    })
    pdf(folder / "文字 PDF.pdf")


def pdf(path, blank=False):
    # Two text pages; the second uses a ToUnicode CMap for Chinese characters.
    chinese = "海底光缆维护"
    cmap = "1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
    cmap += f"{len(chinese)} beginbfchar\n"
    cmap += "\n".join(f"<{i:04X}> <{ord(c):04X}>" for i, c in enumerate(chinese, 1))
    cmap += "\nendbfchar\n"
    stream1 = b"" if blank else b"BT /F1 12 Tf 50 700 Td (Broadband maintenance manual) Tj ET"
    codes = "".join(f"{i:04X}" for i in range(1, len(chinese) + 1))
    stream2 = b"" if blank else f"BT /F2 12 Tf 50 700 Td <{codes}> Tj ET".encode()

    def stream(data):
        return f"<< /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream"

    objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 5 0 R >> >> /Contents 6 0 R >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F2 7 0 R >> >> /Contents 8 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        stream(stream1),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /SimSun /Encoding /Identity-H /DescendantFonts [9 0 R] /ToUnicode 10 0 R >>",
        stream(stream2),
        b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /SimSun /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /DW 1000 /FontDescriptor 11 0 R >>",
        stream(cmap.encode()),
        b"<< /Type /FontDescriptor /FontName /SimSun /Flags 4 /FontBBox [0 -200 1000 900] /ItalicAngle 0 /Ascent 900 /Descent -200 /CapHeight 700 /StemV 80 >>",
    ]
    data = bytearray(b"%PDF-1.4\n")
    offsets = [0]
    for i, obj in enumerate(objects, 1):
        offsets.append(len(data))
        data.extend(f"{i} 0 obj\n".encode() + obj + b"\nendobj\n")
    xref = len(data)
    data.extend(f"xref\n0 {len(offsets)}\n0000000000 65535 f \n".encode())
    for offset in offsets[1:]:
        data.extend(f"{offset:010d} 00000 n \n".encode())
    data.extend(f"trailer << /Size {len(offsets)} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF".encode())
    path.write_bytes(data)


def main():
    binary = Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="doc-search-lite-") as temp:
        root = Path(temp)
        docs = root / "中文 materials"
        docs.mkdir()
        fixtures(docs)
        # Application commands must work without any developer tools in PATH.
        env = dict(os.environ, PATH="")
        db = root / "index.db"

        def run(*args, ok=True):
            result = subprocess.run([str(binary), "--db", str(db), *map(str, args)],
                                    env=env, capture_output=True, text=True, encoding="utf-8", timeout=45)
            assert (result.returncode == 0) == ok, (args, result.stdout, result.stderr)
            return json.loads(result.stdout) if result.stdout.strip() else None

        started = time.perf_counter()
        first = run("index", docs)
        assert first["imported"] == 4 and not first["errors"], first
        cases = [("验收标准", ".docx", "document", "submarine"),
                 ("SUBMARINE", ".docx", "document", "验收"),
                 ("HN-2026", ".docx", "document", "& 合作方"),
                 ("网络建设", ".pptx", "slide 1", "rollout"),
                 ("second presentation", ".pptx", "slide 2", "后续"),
                 ("设备采购", ".xlsx", "row 4", "250000"),
                 ("procurement", ".xlsx", "项目预算", "金额"),
                 ("Broadband", ".pdf", "page 1", "maintenance"),
                 ("海底光缆维护", ".pdf", "page 2", "海底光缆维护")]
        for query, suffix, location, expected in cases:
            results = run("search", query)["results"]
            assert any(r["path"].endswith(suffix) and location in r["location"] and expected in r["text"] for r in results), (query, results)
        assert run("search", "nosuchwordxyz")["results"] == []
        run("search", '" OR *')  # Input is always tokenized, never injected into FTS syntax.
        assert len(run("search", "海底", "--limit", "1")["results"]) == 1
        run("search", "海底", "--limit", "0", ok=False)
        assert run("index", docs)["skipped"] == 4
        assert run("index", docs, "--force")["imported"] == 4
        note = docs / "updates.txt"
        note.write_text("obsoletekeyword " + "数据资料 " * 500, encoding="utf-8")
        assert run("index", docs)["imported"] == 1
        assert all(len(r["text"]) <= 900 for r in run("search", "数据")["results"])
        note.write_text("replacementkeyword 更新后的内容", encoding="utf-8")
        run("index", docs)
        assert not run("search", "obsoletekeyword")["results"]
        assert run("search", "replacementkeyword")["results"]
        note.unlink()
        assert run("index", docs)["removed"] == 1
        assert not run("search", "replacementkeyword")["results"]
        broken = docs / "broken.docx"
        broken.write_bytes(b"not a zip")
        pdf(docs / "扫描件.pdf", blank=True)
        failure = run("index", docs, ok=False)
        assert len(failure["errors"]) == 2, failure
        assert run("status")["documents"] == 4
        # An updated file that can no longer be parsed must not leave stale evidence.
        (docs / "合同 Word.docx").write_bytes(b"broken now")
        run("index", docs, ok=False)
        assert not run("search", "submarine")["results"]
        assert run("remove", docs)["removed"] == 3
        assert run("status")["chunks"] == 0
        assert (docs / "预算 Excel.xlsx").exists()
        print(f"PASS: installed binary, Chinese/English DOCX/XLSX/PPTX/PDF, citations, incremental updates, errors, bounded output ({time.perf_counter()-started:.2f}s)")


if __name__ == "__main__":
    main()
