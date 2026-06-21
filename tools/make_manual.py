# -*- coding: utf-8 -*-
"""Generate manual.pdf — a structured user manual for apitester, with a
clickable table of contents (page numbers + PDF outline)."""

from reportlab.lib.pagesizes import A4
from reportlab.lib.units import cm
from reportlab.lib import colors
from reportlab.lib.styles import getSampleStyleSheet, ParagraphStyle
from reportlab.platypus import (
    BaseDocTemplate, PageTemplate, Frame, Paragraph, Spacer, PageBreak,
    Table, TableStyle, Preformatted,
)
from reportlab.platypus.tableofcontents import TableOfContents

ACCENT = colors.HexColor("#1565C0")
DARK = colors.HexColor("#1a1a1a")
GREY = colors.HexColor("#666666")
CODEBG = colors.HexColor("#f2f2f2")
CODEBORDER = colors.HexColor("#d0d0d0")

styles = getSampleStyleSheet()
body = ParagraphStyle("Body", parent=styles["BodyText"], fontName="Helvetica",
                      fontSize=10.5, leading=15, spaceAfter=6, textColor=DARK)
h1 = ParagraphStyle("H1", parent=styles["Heading1"], fontName="Helvetica-Bold",
                    fontSize=17, leading=21, spaceBefore=18, spaceAfter=8,
                    textColor=ACCENT)
h2 = ParagraphStyle("H2", parent=styles["Heading2"], fontName="Helvetica-Bold",
                    fontSize=12.5, leading=16, spaceBefore=10, spaceAfter=4,
                    textColor=DARK)
toc_title = ParagraphStyle("TOCTitle", parent=h1, spaceBefore=0)
code_style = ParagraphStyle("Code", fontName="Courier", fontSize=9, leading=12,
                            textColor=DARK)
cell = ParagraphStyle("Cell", parent=body, fontSize=9.5, leading=13, spaceAfter=0)
cell_b = ParagraphStyle("CellB", parent=cell, fontName="Helvetica-Bold")
bullet = ParagraphStyle("Bullet", parent=body, leftIndent=14, bulletIndent=2,
                        spaceAfter=3)

story = []
_key = [0]


def heading(text, style):
    """A heading paragraph tagged so afterFlowable can index + bookmark it.
    The bookmark key is fixed at creation so it stays stable across the
    multi-pass TOC build (otherwise reportlab never converges)."""
    p = Paragraph(text, style)
    p._toc_level = 0 if style is h1 else 1
    p._toc_key = "sec%d" % _key[0]
    _key[0] += 1
    return p


def H1(text):
    story.append(heading(text, h1))


def H2(text):
    story.append(heading(text, h2))


def P(text):
    story.append(Paragraph(text, body))


def UL(items):
    for it in items:
        story.append(Paragraph(it, bullet, bulletText="•"))


def CODE(text):
    pre = Preformatted(text, code_style)
    t = Table([[pre]], colWidths=[16.2 * cm])
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), CODEBG),
        ("BOX", (0, 0), (-1, -1), 0.5, CODEBORDER),
        ("LEFTPADDING", (0, 0), (-1, -1), 8),
        ("RIGHTPADDING", (0, 0), (-1, -1), 8),
        ("TOPPADDING", (0, 0), (-1, -1), 6),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
    ]))
    story.append(t)
    story.append(Spacer(1, 6))


def TABLE(rows, widths):
    data = [[Paragraph(c, cell_b) for c in rows[0]]]
    for r in rows[1:]:
        data.append([Paragraph(c, cell) for c in r])
    t = Table(data, colWidths=widths, repeatRows=1)
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, 0), ACCENT),
        ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
        ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
        ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#f6f8fb")]),
        ("GRID", (0, 0), (-1, -1), 0.4, colors.HexColor("#cfd8dc")),
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
        ("LEFTPADDING", (0, 0), (-1, -1), 6),
        ("RIGHTPADDING", (0, 0), (-1, -1), 6),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
    ]))
    story.append(t)
    story.append(Spacer(1, 8))


# ----------------------------------------------------------------------------
# Cover
# ----------------------------------------------------------------------------
story.append(Spacer(1, 5 * cm))
story.append(Paragraph("apitester", ParagraphStyle(
    "Cover", fontName="Helvetica-Bold", fontSize=40, leading=46,
    alignment=1, textColor=ACCENT)))
story.append(Spacer(1, 0.4 * cm))
story.append(Paragraph("Manual Pengguna", ParagraphStyle(
    "Sub", fontName="Helvetica", fontSize=18, alignment=1, textColor=DARK)))
story.append(Spacer(1, 0.3 * cm))
story.append(Paragraph(
    "TUI penguji HTTP API di terminal - alternatif Postman/Insomnia",
    ParagraphStyle("Sub2", fontName="Helvetica-Oblique", fontSize=11,
                   alignment=1, textColor=GREY)))
story.append(Spacer(1, 2.5 * cm))
story.append(Paragraph("Versi 0.1.0", ParagraphStyle(
    "Ver", fontName="Helvetica", fontSize=10, alignment=1, textColor=GREY)))
story.append(PageBreak())

# ----------------------------------------------------------------------------
# Table of contents
# ----------------------------------------------------------------------------
story.append(Paragraph("Daftar Isi", toc_title))
story.append(Spacer(1, 6))
toc = TableOfContents()
toc.levelStyles = [
    ParagraphStyle("TOC1", fontName="Helvetica-Bold", fontSize=11, leading=20,
                   textColor=DARK),
    ParagraphStyle("TOC2", fontName="Helvetica", fontSize=10, leading=16,
                   leftIndent=18, textColor=GREY),
]
story.append(toc)
story.append(PageBreak())

# ----------------------------------------------------------------------------
# 1. Pendahuluan
# ----------------------------------------------------------------------------
H1("1. Pendahuluan")
P("apitester adalah aplikasi terminal (TUI) satu-berkas untuk menguji HTTP API "
  "secara cepat - mirip Postman atau Insomnia, tetapi sepenuhnya di "
  "terminal dan lintas platform (Windows &amp; Linux).")
P("Koleksi request disimpan sebagai berkas TOML yang mudah dibaca manusia, "
  "sehingga bisa diversi dengan git dan diedit langsung bila perlu.")
UL([
    "Kirim request HTTP (GET/POST/PUT/PATCH/DELETE) dan lihat response berwarna.",
    "Environment + interpolasi variabel <font face='Courier'>{{var}}</font>.",
    "Riwayat request dan cookie jar yang tersimpan permanen.",
    "Pencarian di dalam body response, ekspor, dan salin ke clipboard.",
    "Mode headless untuk skrip/CI.",
])

# ----------------------------------------------------------------------------
# 2. Instalasi & Menjalankan
# ----------------------------------------------------------------------------
H1("2. Instalasi &amp; Menjalankan")
P("Bangun dari sumber dengan Rust (cargo), lalu jalankan binarinya dengan "
  "menunjuk ke sebuah berkas koleksi:")
CODE("cargo build --release\n"
     ".\\target\\release\\apitester.exe .\\collections\\example.toml")
P("Argumen <font face='Courier'>.toml</font> bersifat opsional - tanpa "
  "argumen aplikasi terbuka dengan koleksi kosong (tekan "
  "<font face='Courier'>a</font> untuk menambah request).")

# ----------------------------------------------------------------------------
# 3. Antarmuka
# ----------------------------------------------------------------------------
H1("3. Antarmuka")
P("Layar terbagi menjadi tiga panel utama, ditambah baris status dan baris "
  "bantuan tombol di bawah:")
CODE(
    "+------------+-------------------------------------+\n"
    "| collections| request                             |\n"
    "|  Get Status|   Method : GET   [m] cycle          |\n"
    "|  Echo JSON |   URL    : /status/200              |\n"
    "|  Query ... |   Query/Headers/Body ...            |\n"
    "|            +-------------------------------------+\n"
    "|            | response                            |\n"
    "|            |   Status: 200 OK  Time  Size        |\n"
    "|            |   { ...JSON berwarna... }           |\n"
    "+------------+-------------------------------------+\n"
    " [j/k] nav  [s] send  [/] find  [H] history  [?] help")
UL([
    "<b>collections</b> (kiri): daftar request; method diberi warna.",
    "<b>request</b> (kanan atas): method, URL, query, headers, body.",
    "<b>response</b> (kanan bawah): status, waktu, ukuran, dan body.",
])

# ----------------------------------------------------------------------------
# 4. Navigasi & Keybinding
# ----------------------------------------------------------------------------
H1("4. Navigasi &amp; Keybinding")
H2("4.1 Mode normal")
TABLE([
    ["Tombol", "Fungsi"],
    ["Up/Down atau j / k", "Pindah pilihan / scroll response"],
    ["Tab", "Pindah panel (collections -> request -> response)"],
    ["s", "Kirim request yang dipilih"],
    ["Esc", "Batalkan request yang sedang berjalan"],
    ["m", "Ganti method (GET -> POST -> PUT -> PATCH -> DELETE)"],
    ["a / d", "Tambah / hapus request (hapus dikonfirmasi)"],
    ["e / E", "Edit field aktif / edit body di $EDITOR"],
    ["w", "Simpan koleksi ke berkasnya"],
    ["h", "Tampil/sembunyikan response headers"],
    ["/ , n , N", "Cari di body; match berikutnya / sebelumnya"],
    ["o / y", "Ekspor body ke berkas / salin ke clipboard"],
    ["H", "Lihat riwayat request"],
    ["?", "Bantuan"],
    ["q / Ctrl-C", "Keluar (konfirmasi bila ada perubahan)"],
], [5.4 * cm, 10.8 * cm])

H2("4.2 Mode edit (insert)")
P("Ditandai dengan badge INSERT di baris bawah. Masuk dengan "
  "<font face='Courier'>e</font> pada field yang dipilih:")
UL([
    "<b>Esc</b> menyimpan dan keluar untuk semua field.",
    "Untuk <b>URL</b>, <b>Enter</b> juga menyimpan; pada field multi-baris "
    "Enter menyisipkan baris baru.",
    "<b>Headers</b> ditulis <font face='Courier'>Key: value</font> per baris.",
    "<b>Query</b> ditulis <font face='Courier'>key=value</font> per baris.",
])

# ----------------------------------------------------------------------------
# 5. Alur Kerja Dasar
# ----------------------------------------------------------------------------
H1("5. Alur Kerja Dasar")
P("Langkah paling umum untuk mengirim sebuah request:")
UL([
    "Pilih request di panel collections dengan <font face='Courier'>j</font>/"
    "<font face='Courier'>k</font>.",
    "Tekan <font face='Courier'>s</font> untuk mengirim.",
    "Lihat hasilnya di panel response: baris status berwarna (hijau = 2xx, "
    "kuning = 4xx, merah = 5xx), waktu, ukuran, lalu body.",
    "Body JSON otomatis di-<i>pretty-print</i> dan diberi syntax highlight.",
])

# ----------------------------------------------------------------------------
# 6. Mengedit Request
# ----------------------------------------------------------------------------
H1("6. Mengedit Request")
P("Pindah ke panel request (<font face='Courier'>Tab</font>), pilih field "
  "dengan panah, lalu:")
UL([
    "<font face='Courier'>m</font> untuk memutar method.",
    "<font face='Courier'>e</font> untuk mengedit URL, body, headers, atau "
    "query secara inline.",
    "<font face='Courier'>E</font> untuk membuka body di editor eksternal "
    "(<font face='Courier'>$VISUAL</font>/<font face='Courier'>$EDITOR</font>, "
    "fallback notepad/vi).",
    "<font face='Courier'>w</font> untuk menyimpan perubahan ke berkas "
    "<font face='Courier'>.toml</font>. Indikator <font face='Courier'>&#9679; "
    "dirty</font> menandai perubahan yang belum tersimpan.",
])

# ----------------------------------------------------------------------------
# 7. Bekerja dengan Response
# ----------------------------------------------------------------------------
H1("7. Bekerja dengan Response")
TABLE([
    ["Tombol", "Fungsi"],
    ["h", "Tampil/sembunyikan response headers"],
    ["/ lalu ketik", "Cari teks di body (tidak peka huruf besar/kecil)"],
    ["n / N", "Lompat ke match berikutnya / sebelumnya"],
    ["o", "Ekspor body ke <berkas>.json|txt"],
    ["y", "Salin body ke clipboard"],
    ["Up/Down", "Scroll body saat panel response aktif"],
], [5.4 * cm, 10.8 * cm])

# ----------------------------------------------------------------------------
# 8. Riwayat Request
# ----------------------------------------------------------------------------
H1("8. Riwayat Request")
P("Setiap request yang dikirim (lewat TUI maupun headless) dicatat permanen. "
  "Tekan <font face='Courier'>H</font> untuk membuka daftar riwayat "
  "(terbaru di atas), lengkap dengan waktu, method, nama, status, dan durasi.")
P("Riwayat disimpan di <font face='Courier'>history.jsonl</font> pada direktori "
  "data (lihat bab 12).")

# ----------------------------------------------------------------------------
# 9. Format File Koleksi (TOML)
# ----------------------------------------------------------------------------
H1("9. Format File Koleksi (TOML)")
H2("9.1 Struktur dasar")
CODE('name     = "API Saya"\n'
     'base_url = "https://httpbin.org"\n\n'
     '[[requests]]\n'
     'name   = "Cek user"\n'
     'method = "GET"\n'
     'url    = "/get"            # relatif -> digabung ke base_url\n'
     'query  = { page = "1" }')

H2("9.2 Environment &amp; interpolasi")
P("<font face='Courier'>[env.default]</font> adalah basis; "
  "<font face='Courier'>--env prod</font> menimpanya dengan "
  "<font face='Courier'>[env.prod]</font>. Tulis "
  "<font face='Courier'>{{var}}</font> di URL, headers, query, atau body untuk "
  "disubstitusi; variabel yang tak terdefinisi dianggap error.")
CODE('[env.default]\n'
     'token = "Bearer dev-123"\n\n'
     '[env.prod]\n'
     'token = "Bearer prod-xyz"\n\n'
     '[[requests]]\n'
     'name    = "Kirim data"\n'
     'method  = "POST"\n'
     'url     = "/post"\n'
     'headers = { Authorization = "{{token}}" }')

H2("9.3 Tipe body")
P("<font face='Courier'>[requests.body]</font> menerima "
  "<font face='Courier'>type</font>: json, form, xml, text (atau raw). "
  "Content-Type diturunkan otomatis dari tipe kecuali kamu menyetelnya sendiri.")
CODE('[requests.body]\n'
     'type    = "json"\n'
     "content = '''\n"
     '{ "hello": "world" }\n'
     "'''")

H2("9.4 Multipart / upload file")
P("Setel <font face='Courier'>type = \"multipart\"</font> lalu daftarkan "
  "<font face='Courier'>[[requests.body.parts]]</font>. Tiap part adalah field "
  "teks (<font face='Courier'>value</font>) atau berkas "
  "(<font face='Courier'>file</font>). <font face='Courier'>filename</font> dan "
  "<font face='Courier'>content_type</font> opsional; Content-Type (dengan "
  "boundary) disetel otomatis. <font face='Courier'>{{var}}</font> berlaku di "
  "nama, value, path, dan filename.")
CODE('[requests.body]\n'
     'type = "multipart"\n\n'
     '[[requests.body.parts]]\n'
     'name  = "greeting"\n'
     'value = "hello"\n\n'
     '[[requests.body.parts]]\n'
     'name         = "doc"\n'
     'file         = "{{home}}/report.pdf"\n'
     'filename     = "report.pdf"\n'
     'content_type = "application/pdf"')

# ----------------------------------------------------------------------------
# 10. Mode Headless
# ----------------------------------------------------------------------------
H1("10. Mode Headless")
P("Jalankan satu request tanpa UI, cetak response ke stdout, lalu keluar. "
  "Cocok untuk skrip dan CI:")
CODE('.\\target\\release\\apitester.exe .\\punyaku.toml --headless "Cek user"')
P("Diagnostik (panah request/response) ditulis ke stderr; body (pretty) ke "
  "stdout, sehingga mudah dipipa ke alat lain.")

# ----------------------------------------------------------------------------
# 11. Opsi CLI
# ----------------------------------------------------------------------------
H1("11. Opsi CLI")
TABLE([
    ["Flag", "Keterangan"],
    ["-e, --env &lt;ENV&gt;", "Environment untuk interpolasi (default: default)"],
    ["-t, --timeout &lt;DETIK&gt;", "Timeout request (default: 30)"],
    ["-k, --insecure", "Lewati verifikasi sertifikat TLS"],
    ["--no-redirect", "Jangan ikuti redirect"],
    ["--proxy &lt;URL&gt;", "Proxy HTTP/HTTPS (env HTTP_PROXY dll. tetap dihormati)"],
    ["--no-color", "Matikan warna ANSI (dan syntax highlight)"],
    ["--theme &lt;dark|light&gt;", "Tema syntax highlight (default: dark)"],
    ["--no-cookies", "Matikan cookie jar persisten"],
    ["--headless &lt;NAMA&gt;", "Jalankan satu request non-interaktif, lalu keluar"],
], [5.6 * cm, 10.6 * cm])

# ----------------------------------------------------------------------------
# 12. State Persisten
# ----------------------------------------------------------------------------
H1("12. State Persisten")
P("State disimpan di direktori data platform:")
UL([
    "Linux: <font face='Courier'>~/.local/share/apitester/</font>",
    "macOS: <font face='Courier'>~/Library/Application Support/apitester/</font>",
    "Windows: <font face='Courier'>%APPDATA%\\apitester\\data\\</font>",
])
P("Setel <font face='Courier'>APITESTER_DATA_DIR</font> untuk menimpa lokasinya. "
  "Berkas: <font face='Courier'>history.jsonl</font> (riwayat) dan "
  "<font face='Courier'>cookies.json</font> (cookie jar).")

# ----------------------------------------------------------------------------
# 13. Kode Keluar
# ----------------------------------------------------------------------------
H1("13. Kode Keluar (Headless)")
TABLE([
    ["Kode", "Arti"],
    ["0", "Sukses (status &lt; 400)"],
    ["1", "Status HTTP error (&gt;= 400)"],
    ["2", "Tidak ada berkas koleksi diberikan"],
    ["3", "Nama request tidak ditemukan"],
    ["4", "Error transport (timeout/DNS/koneksi)"],
], [3.2 * cm, 13 * cm])

# ----------------------------------------------------------------------------
# 14. Tips & Pemecahan Masalah
# ----------------------------------------------------------------------------
H1("14. Tips &amp; Pemecahan Masalah")
UL([
    "<b>TLS error</b> pada host tepercaya: tambahkan "
    "<font face='Courier'>-k</font>/<font face='Courier'>--insecure</font>.",
    "<b>Timeout</b>: naikkan <font face='Courier'>--timeout</font> bila server "
    "lambat.",
    "<b>Variabel undefined</b>: pastikan ada di <font face='Courier'>[env.*]</font> "
    "dan environment yang aktif benar (<font face='Courier'>--env</font>).",
    "<b>Multipart gagal baca file</b>: gunakan path absolut yang valid untuk OS "
    "kamu (di Windows: <font face='Courier'>C:/path/ke/berkas</font>).",
    "<b>Tema kurang kontras</b>: coba <font face='Courier'>--theme light</font> "
    "untuk terminal latar terang.",
])
story.append(Spacer(1, 12))
P("<i>Dokumen ini dibuat otomatis dari fitur apitester versi 0.1.0.</i>")


# ----------------------------------------------------------------------------
# Doc template with header/footer, TOC notify, and PDF outline bookmarks
# ----------------------------------------------------------------------------
def footer(canvas, doc):
    canvas.saveState()
    canvas.setFont("Helvetica", 8)
    canvas.setFillColor(GREY)
    if doc.page > 1:
        canvas.drawString(2 * cm, 1.1 * cm, "apitester - Manual Pengguna")
        canvas.drawRightString(A4[0] - 2 * cm, 1.1 * cm, "Hal. %d" % doc.page)
    canvas.restoreState()


class ManualDoc(BaseDocTemplate):
    def afterFlowable(self, flowable):
        level = getattr(flowable, "_toc_level", None)
        if level is None:
            return
        text = flowable.getPlainText()
        key = flowable._toc_key
        self.canv.bookmarkPage(key)
        self.canv.addOutlineEntry(text, key, level=level, closed=False)
        self.notify("TOCEntry", (level, text, self.page, key))


doc = ManualDoc("manual.pdf", pagesize=A4,
                leftMargin=2 * cm, rightMargin=2 * cm,
                topMargin=2 * cm, bottomMargin=2 * cm,
                title="apitester - Manual Pengguna", author="apitester")
frame = Frame(doc.leftMargin, doc.bottomMargin, doc.width, doc.height, id="n")
doc.addPageTemplates([PageTemplate(id="main", frames=[frame], onPage=footer)])
doc.multiBuild(story)
print("OK: manual.pdf")
