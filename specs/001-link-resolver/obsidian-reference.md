# Obsidian reference

This document contains Obsidian.md reference (OR) as 2026-09-21 from [official help website](https://obsidian.md/help).

All rules have OR_ identifier for reference. They are all **bold**, aka between double star (`**`).

## **OR1** File Format

> [Source](https://obsidian.md/help/file-formats)

Here a list of the official files extention supported by Obsidian. 

- Markdown: `.md`
- Bases: `.base`
- JSON Canvas: `.canvas` [Learn more](https://jsoncanvas.org/)
- Images: `.avif`, `.bmp`, `.gif`, `.jpeg`, `.jpg`, `.png`, `.svg`, `.webp`
- Audio: `.flac`, `.m4a`, `.mp3`, `.ogg`, `.wav`, `.webm`, `.3gp`
- Video: `.mkv`, `.mov`, `.mp4`, `.ogv`, `.webm`
- PDF: `.pdf`

Any other extention cannot be linked.

The first one (`.md`) are called note and the others attachements.

### **OR2** Extend supported file by plugin

> [Source](https://obsidian.md/help/file-formats)

Plugin can extend supported files, [see](https://obsidian.md/help/community-plugins).

### **OR3** dot file

This is not formally documented, but the application do not allowed files beginning by a dot (`.`). We can imlplied that they are internal use only and cannot be refrenced in a link.

### **OR4** dot folder

This is not formally documented, but the application do not allowed folfer beginning by a dot (`.`). We can implied that any content or the folder itself cannot be refrenced in a link. _They must be for internal use_.

### **OR5** invalid character in file and folder

Those are invalid character in file and folder name: `*`, `"`, `/`, `\`, `<`, `>`, `:`, `|`, `?`, `#`, `%%`, `[`, `]`, `^`.

## Link

Link in used to referece a note (entirely, one of its heading, one of its block) on an attachment (entirely only).

### **OR6** Format 

- **Wikilink:** `[[...]]` (**OR6.1**)
  - Each part of the link are separated by `|`. Only the first is mandatory and the separator is omited if the right part is unspecified (**OR6.2**)
- **Markdown:** `[...](...)` (**OR6.3**)
  - 2 part (left and right) are explicit and mandatory.

### **OR7** Reference to file name

Both format can specify the file name without folder path. For note file (`.md`), the extention is optional in both format (**OR7.1**)

- **Wikilink:** This is specified in the _first part_.
- **Markdown:** This is the _right parth_.
  - Special chararter must be URL encoded (**OR7.2**)

### **OR8** Reference with path

The file name can be prefixed with the path relative to the vault root. Folder separator used is `/` on any OS. OR7 rules still applied.

### **OR9** Link to heading

In note file (`.md`), the link can reference a heading (line starting with 1 or more `#`).

After the file name (see OR7) add one hashtag (`#`) fallowed by the text of the heading. Nomatter the heading level, only one hashtag separate the heading reference of the file name.

This is supported in both format. In **Markdown**, the hashtag is not URL encoded but the heading text is (**OR9.1**).

#### **OR10** Same file heading

If the referenced heading is in the same file note of the link, the file name can be ommited and directly specified the headding (starting with the hashtag)

#### **OR11** Link with Subheadings

To reference a sub heading in a heading use `#` as heading separator from the highest level (lesser count of `#` in heading) to the lowest (greated count of `#` in heading)

### **OR12** Link to a block

Lines that ends with `^...` where the part after the caret is a text identifier can be reference. They can be reference like heading unsing `#^...`.

#### **OR12.1** Structured blocks with block identifier

For _Structured blocks_ (lists, quotations, callouts, tables, Math), the block identifier should be on a separate line, with a blank line before and after.

Note, list item can still be referenced by adding block identifier to the end of the line.

Personal experimentation for quotations and callouts, the block indentifier can be place a the end of the last line of the structured block (**OR12.2**).

### **OR13** Display text

A display text can be specied to a link. This impact only rendering and have no effect on the reference definition.

- **Wikilink:** This is specified in the _second part_ after a `|`.
- **Markdown:** This is the _left parth_.

### **OR14** Empeded link

The link can start with an exclamation mark (`!`) (before the `[[` in wikilink of the `[` in markdown) to indicate that the link is empeded. This only impact rendering.
