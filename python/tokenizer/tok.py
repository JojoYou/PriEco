import string
from nltk.stem.snowball import SnowballStemmer
from langdetect import detect
import sys
from flask import Flask, request

language_map = {
    'af': 'afrikaans',
    'sq': 'albanian',
    'ar': 'arabic',
    'hy': 'armenian',
    'eu': 'basque',
    'ca': 'catalan',
    'da': 'danish',
    'nl': 'dutch',
    'en': 'english',
    'et': 'estonian',
    'fi': 'finnish',
    'fr': 'french',
    'de': 'german',
    'el': 'greek',
    'hi': 'hindi',
    'hu': 'hungarian',
    'id': 'indonesian',
    'ga': 'irish',
    'it': 'italian',
    'la': 'latin',
    'lv': 'latvian',
    'lt': 'lithuanian',
    'no': 'norwegian',
    'pt': 'portuguese',
    'ro': 'romanian',
    'ru': 'russian',
    'gd': 'scottish',
    'es': 'spanish',
    'sv': 'swedish',
    'tr': 'turkish',
}

def tokenize_query(query):
    stopwords_path = '/var/www/html/prieco/python/tokenizer/stopwords.txt'
    with open(stopwords_path, 'r', encoding='utf-8') as f:
        stop_words = set(word.strip().lower() for word in f.readlines())

    tokens = query.split()

    tokens = [''.join(char for char in token if char not in string.punctuation) for token in tokens]
    tokens = [token for token in tokens if token.lower() not in stop_words]

    language = detect(query)
    language = language_map.get(language, 'english')
    stemmer = SnowballStemmer(language)
    tokens = [stemmer.stem(token) for token in tokens]
    
    return tokens


app = Flask(__name__)
@app.route("/", methods=['GET'])
def index():
    query =  request.args.get('txt')
    tokens = ' '.join(tokenize_query(query))
    return tokens

app.run(host="0.0.0.0", port=8081)