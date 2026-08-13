# **Layout**

PriEco consists of these main parts

## **PriEco (website/node)**

### **Core**

Contains global constants, objects and functions

### [**Mini crawler**](../minicrawler/)

A node runs page fetcher and parser

It receives URLs from [main crawler](#crawler)

Downloads them

Extracts the same data as [main crawler](#crawler)

Sends them to [main crawler](#crawler) which then sends mini crawler new URLs for fetching and finishes the results

### **Blob storage**

Manages dompressed blobs insertion and decompression

### **Inserter**

Inserts results made by crawler to index

It inserts data to

- FTS (Tantivy) index
- Vector (IVF) index
- Meta storage: A key-value embedded storage

Handles data integrity, commits to disks and merges

### **Web**

_The website and search_

Handles all PriEco endpoints. We use [Rocket.rs](https://rocket.rs/) and index search pipeline

Index consists of

**Direct search**

- Checks for domain results of .com, .net, .org where domain is trimmed query.

- Helpful for root domain searches such as YouTube.

**Discovery search**

- Pings a lot of domains matching query. And waits (max 1s) for 200 response code.

- Useful for root domain searches that PriEco doesn't yet know.

- Successful findings are sent to PriEco web crawler.

**Full-text search**

- PriEco uses [Tantivy](https://github.com/quickwit-oss/tantivy).

- Tantivy checks for keywords in web page `title`, `description`, `content` (first 500 page characters), `keywords` (manually set keywords by page).

- Many rules are applied during this search, especially with query intent and preferred language.

**Vector search**

- IVF index made by me. I needed a simple vector search that embeds query, finds closest centropoids, mmap sequencially reads those buckets and gets the closest vectors to the query in cosine similarity.

- Vectors are normalized to length of 1 for faster math.

### **PageRank**

Was used to calculate PageRank scores. Is used to retrieve them.

A new non-flawed implementation must be made.

## **Browser extension**

_Mostly upcoming part_

User privacy and security are going to be top priority

Everything opt-in by default

Would consist of

- Mini crawler: just as PriEco node
- Web Discovery: Visited URLs double fetched, parsed and sent to main crawler
- Query log: Scrapping query + results out of other web search engines to help improve PriEco ranking

## **Crawler**

Currently, proprietary program running on PriEco main node

Its purpose is to:

- Manage crawling queue

- Organize crawling nodes and extensions

- Download web pages

- Extract from them:

- Title

- Description

- Content: first max 500 characters

- Keywords (meta tag)

- URL of first image

- Date of publication (fallback is date of crawling)

- Detect website country by IP

- Detect language using [whatlang](https://crates.io/crates/whatlang) crate using page text, fallback is language of hosted country

- And to blobs extract

- h1..h6 text

- "p", "span", "a", "li", "label" text

- a href links

- img URLs

- meta tags data

- The part between "Organize nodes" and this are done by nodes and extension too

- Calculate page scores

- Pass page links to queue

- Classify page intent

- Create page embeds
