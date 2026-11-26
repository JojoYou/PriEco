# **Blob Storage**

Stores compressed text, tags and links of crawled pages

Meta storage only holds what a result needs (title, description, first 500 characters...). Blob storage holds the rest of the page

## **Insert**

Crawler sends blobs as `.tar.gz` files

Every minute PriEco

- Finds all directories in the import directory
- Gets their `.tar.gz` files
- Streams files out of them, only `.zst` and `.txt` are accepted
- Inserts them to blob storage

## **Format**

A blob isn't html. It's a list of tags and their words

- First byte says if the rest is zstd compressed
- Then tags, each one has
    - Tag byte: h1..h6, p, span, a, li, label, meta, links, images...
    - Words: length byte + the word
    - End marker `255 255 255 255`

Short words are stored as they are, longer ones as an ID of a word in a dictionary

This keeps blobs small

## **Decode**

A blob can be read back as

- Text: `<tag>words</tag>`, one tag per line
- Html: readable page, images go through PriEco proxy

Pages deleted from the index are deleted from blob storage too
