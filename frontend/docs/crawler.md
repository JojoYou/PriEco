# **Crawler**

Currently, proprietary program running on PriEco main node

Nodes help it with [mini crawler](../minicrawler/)

## **Purpose**

- Manage crawling queue
- Organize crawling nodes and extensions
- Download web pages
- Extract data from them
- Calculate page scores
- Pass page links to queue
- Classify page intent
- Create page embeds

## **Extracted data**

Downloading and extracting is done by nodes and extension too

- Title: `og:site_name`, then `title`, then first h1..h6. Max 60 characters
- Description: `og:description`, then meta description, then first p. Max 160 characters
- Content: first 500 characters of h2..h6 and p
- Keywords (meta tag), favicon, URL of first image
- Date of publication, fallback is date of crawling
- Country: domain → IP → country
- Language: detected using [whatlang](https://crates.io/crates/whatlang) crate from page text
- NSFW: word lists for URL and text

And for [blobs](../blob/)

- h1..h6 text
- "p", "span", "a", "li", "label" text
- a href links
- img URLs
- meta tags data

## **Scores**

- Confidence: how easy the page is to read
- Effort: how much work was put into the text
- Load time
- Mobile friendly
- Has 500+ words

## **Output**

Results are sent to nodes as

- `.zip` files for [Inserter](../inserter/)
- `.tar.gz` files for [Blob storage](../blob/)
