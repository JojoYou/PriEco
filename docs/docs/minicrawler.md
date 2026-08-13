# **Mini Crawler**

An async mini crawler

## Pipeline

Fetches URLs from main crawler

Downloads them in parallel

- Node instructs Mini crawler how many concurrent downloads it should use

Parses document the same way as main crawler

- Extracts text, a + img links and meta data
- Extracts title, description...

Buffers them

Async asks for new URLs and sends the data to main crawler
