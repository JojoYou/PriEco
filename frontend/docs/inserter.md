# **Data Inserter**

Inserts results made by crawler to index

And keeps the index up to date

## **Insert**

Crawler sends results as `.zip` files

Every minute inserter

- Collects `.zip` files
- Checks disk limit, if index is bigger than `PRIECO_MAX_STORAGE` (GB), files are removed without insertion
- Reads every `.txt` line as one page
- Skips pages already in index and lines with wrong vector length

One page goes to 3 places

- Meta storage: zstd compressed page data, key is ID of URL
- FTS (Tantivy) index: searchable fields
- Vector (IVF) index: vectors are buffered in RAM, then assigned to their bucket on GPU and appended to staging files

Every processed `.zip` file is removed

### Line format

Fields are split by `<-->`

`url, title, description, content, favicon, image, keywords, safe_s, html, lang, loc, [confidence, effort, qna, sts], load, date, intent, is_mobile, has_500_words, vector`

## **Merge**

After insertion, unless `runtime/dont_merge.txt` exists

- Tantivy segments are merged into one
- Meta storage is compacted
- Staging files are merged into `.bin.zst` buckets

Newer vector of a page replaces the old one

Buckets are written to `.tmp` and renamed, so a crash never leaves a half written bucket

## **Updates**

Rules applied to pages already in index. Every hour

Currently one rule: **Dead links**

- Page is pinged 3 times. 2xx, 401 and 403 count as alive
- If it's dead, its domain is pinged 2 times
- Dead domain is remembered, its other pages are removed without ping
- Dead page is removed from meta storage, blob storage and Tantivy

How it works

- New rule gets the current max page ID, pages inserted later already follow it
- All rules run on the same pages at once, so meta storage is read only once
- Changes are written every 1000 pages
- Progress is saved to `runtime/config/index_updates.json`, so updating continues after restart
