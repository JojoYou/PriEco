# **Core System**

Contains global constants, objects and functions

Every other PriEco part uses it

## **Parts**

### **Constants**

Paths and sizes

Most files PriEco makes are in `runtime/`

### **Globals**

Objects shared by the whole project. Most of them are created on first use

- Config of the node
- HTTP clients: normal one, one for proxy and one for pinging websites
- Meta storage and blob storage
- Index: Tantivy + meta storage + IVF vectors
- Models: query embedder and reranker
- Query understanding: synonyms, intent keywords, entities, spell checker, bangs
- PageRank scores
- Analytics
- Terminal colors and icons

### **Helpers**

Small file, URL and hashing functions

URL is hashed to a 64 bit ID, it's used as a key everywhere

### **Set up**

On first run, PriEco asks 7 questions

- IP and port of the website
- Where Tantivy, meta, vector and blob storage are
- Number of workers

Every question can be answered by env variable instead, useful for Docker

- `PRIECO_IP`, `PRIECO_PORT`, `PRIECO_TANTIVY`, `PRIECO_META`, `PRIECO_VECTOR`, `PRIECO_BLOB`, `PRIECO_WORKERS`

Answers are saved to `runtime/config/settings.conf` and loaded on next start

## **Background services**

When the node starts, each enabled service gets its own thread

- [Blob storage](../blob/): every minute
- [Inserter](../inserter/): every minute
- PageRank: every 3 hours
- [Mini crawler](../minicrawler/)
- Index updater: every hour

For full details see the [API reference](../api/prieco_core/index.html)
