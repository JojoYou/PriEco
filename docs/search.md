# **Web Search**

PriEco is an independent web search engine. This means we operate out own index.
And don't rely on 3rd party ones.

## **Pipeline**

### Bang

If query contains Bang or MultiBang, search pipeline is immediately stopped to not waste resources and other back-end function or JavaScript on user side handles the redirect/s

### Cache

Disk I/O is the most expensive part of search and so we hold a RAM Cache that holds the most recent searches. It helps improve search speed a lot, especially when user (or a different user) revisits the same SERP that has been already searched for

Example is an No-JS user (where browser caching is much more limited) coming page from a page to SERP

### Spell checker

User query is first passed to a spell checker which tries its best to correct the words

We don't yet have the resources to use n-grams, they would take Terabytes of storage and a hustle to search through them.

It uses also only English dictionary, for now. This point isn't that hard to fix

Current decision is to suggest to the user a corrected query but search their original query. Other web search engines

### Query intent classification

- We use the same categories as Google

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryIntent {
    Informational,           // Learn information
    Transactional,           // Action purchase, download, sign up
    CommercialInvestigation, // Research, comparing options
    Navigational,            // Reach a known domain, page
    Local,                   // Near me businesses
    Unknown,                 // Failed to determine
}
```

- Categorization is fully performed by Heuristics, for now

### Coordinates

If query contains a place, we extract its coordinates. This is going to be used to show up map widget

### Synonym expansion

This is done only for Full-text search index. Vector index sometimes gets added preferred location for better matching

Synonym expansion works for all PriEco supported languages and helps PriEco find useful results even when user used different keywords than a (high quality) web page

Your query is 3 times more likely to show up in results than synonyms but it helps having them there

### Embedding

To not just rely on keywords and have semantic search PriEco generates vectors out of web pages and at this point in the pipeline, from user query too

### Search

At this point we are ready to perform a search

PriEco does 4 parallel searches at the same time

These are:

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

- IVF index made by me. I needed a simple vector search that embeds query, finds closest centropoids, mmap sequentially reads those buckets and gets the closest vectors to the query in cosine similarity.

- Vectors are normalized to length of 1 for faster math.

### Reciprocal Rank Fusion merge

We now have 4 sorted vectors of results from 4 different calls. We can't work like that, we need one

RRF merge, merges them to a single vector

I find a simple explanation to be [here](https://medium.com/@devalshah1619/mathematical-intuition-behind-reciprocal-rank-fusion-rrf-explained-in-2-mins-002df0cc5e2a)

A part of this merge is deduplication too, as we don't want to have the same URL in results multiple times

### Goggle Discard filter

Goggles can contain blocking of domains or block by default (this is for unmentioned domains in the filter)

We want to remove results based on this filter

### Hand ranking

It's called this way as it's a hand picked set of ranking factors

To improve this ranking a Mwmbl NDCG test is used to improve weights

### PageRank

PageRanks were calculated and we return the scores for the top 100 results, add them in and sort

The PageRanks aren't perfectly precise and aren't updated

_Needs to be properly made again_

### Reranker

We take the best 30 results and pass them to Reranker (BGE model) with query

It reorders the results based on what it thinks are actually the best results for the query

I found it improves results quality a lot

### Final

We trim SERP to 20 results and return them to the user
