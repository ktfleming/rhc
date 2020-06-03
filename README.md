# rhc: Command-line HTTP client

## Introduction

`rhc` is a command-line tool for storing and quickly dispatching HTTP requests. Perhaps the most similar well-known tool is [Postman](https://www.postman.com/), although rhc has only a fraction of Postman's features. On the other hand, it fits well into a command-line/terminal-centric workflow, and is designed to allow you to select and dispatch a desired request as quickly as possible.

## Installation

(TODO)

## Usage

### Request Definitions

#### Basics
Using rhc requires at least one "request definition" file. This type of file is in [TOML](https://github.com/toml-lang/toml) format and contains information about a single HTTP request you want to send (the URL, method, body, etc). As an example, try placing the following content at `~/rhc/definitions/test.toml`:

```toml
[request]
url = "https://httpbin.org/get"
method = "GET"
```

Then try running `rhc -f ~/rhc/definitions/test.toml`. rhc will send a GET request to `https://httpbin.org/get`, and you should see the response, including the status code, headers, and body printed to stdout.

#### Headers
You can specify headers under a `headers` table, like so:
```toml
[request]
url = "https://httpbin.org/get"
method = "GET"

[headers]
headers = [
  { name = "Accept-Language", value = "fr-CH, fr;q=0.9, en;q=0.8, de;q=0.7, *;q=0.5" },
  { name = "Authorization", value = "Bearer xyz" }
]
```

#### Body
You can specify a request body as plain text, a JSON value or URL-encoded data. You must specify which of these body types you want to use under the `body.type` key, and the body itself under the `body.content` key.

```toml
[request]
url = "https://httpbin.org/post"
method = "POST"

[body]
type = "text"
content = "Some plain text"
```

```toml
[request]
url = "https://httpbin.org/post"
method = "POST"

[body]
type = "json"
content = '''
{
  "some_key": "some value",
  "a_number": 123,
  "nested": {
    "inside": true,
    "other": null
  }
}'''
```

```toml
[request]
url = "https://httpbin.org/post"
method = "POST"

[body]
type = "urlencoded"
content = [
  { name = "key1", value = "value1" },
  { name = "あいうえお", value = "猪" }
]
```

The `Content-Type` header on the request will automatically be set to `text/plain`, `application/json`, or `application/x-www-form-urlencoded`, respectively. Note that for a JSON body, it is recommended to use [multi-line literal strings](https://github.com/toml-lang/toml#string) (triple single-quotes) to wrap the raw JSON value. This way you can use double quotations to place JSON strings in the body.
