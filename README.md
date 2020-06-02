# rhc: Command-line HTTP client

## Introduction

`rhc` is a command-line tool for storing and quickly dispatching HTTP requests. Perhaps the most similar well-known tool is [Postman](https://www.postman.com/), although rhc has only a fraction of Postman's features. On the other hand, it fits well into a command-line/terminal-centric workflow, and is designed to allow you to select and dispatch a desired request as quickly as possible.

## Installation

(TODO)

## Usage

### Request Definitions

Using rhc requires at least one "request definition" file. This type of file is in [TOML](https://github.com/toml-lang/toml) format and contains information about a single HTTP request you want to send (the URL, method, body, etc). As an example, try placing the following content at `~/rhc/definitions/test.toml`:

```toml
[request]
url = "https://httpbin.org/get"
method = "GET"
```

Then try running `rhc -f ~/rhc/definitions/test.toml`. rhc will send a GET request to `https://httpbin.org/get`, and you should see the response, including the status code, headers, and body printed to stdout.
