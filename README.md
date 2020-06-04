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

#### Request
The `request` table in the request definition file species the method and URL to use. Both are required. 

Valid values for the `method` key are "GET", "POST", "PUT", "DELETE", "HEAD", "OPTIONS", "PATCH", and "TRACE".

#### Query Parameters
You can specify query parameters in the `query` table:

```toml
[request]
url = "https://httpbin.org/get"
method = "GET"

[query]
params = [
  { name = "id", value = "12345" }
]
```

Alternatively, you can specify them directly in `request.url`:

```toml
[request]
url = "https://httpbin.org/get?id=12345"
method = "GET"
```

#### Headers
You can specify headers under a `headers` table:
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

# Plain text body
[body]
type = "text"
content = "Some plain text"
```

```toml
[request]
url = "https://httpbin.org/post"
method = "POST"

# JSON body
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

# URL-encoded body
[body]
type = "urlencoded"
content = [
  { name = "key1", value = "something" },
  { name = "あいうえお", value = "猪" }
]
```

The `Content-Type` header on the request will automatically be set to `text/plain`, `application/json`, or `application/x-www-form-urlencoded`, respectively. Note that for a JSON body, it is recommended to use [multi-line literal strings](https://github.com/toml-lang/toml#string) (triple single-quotes) to wrap the raw JSON value. This way you can use double quotations to place JSON strings in the body.

Note that `multipart/form-data` requests are not directly supported.

#### Metadata
An optional `metadata` table can provide extra information about the request definition that isn't actually used when sending the request. Currently there is one possible metadata key, which is `description`. This optional description will be displayed in rhc's interactive mode (to be explained later in this document).

```toml
[metadata]
description = "GET example using httpbin"

[request]
url = "https://httpbin.org/get"
method = "GET"
```

#### Variables
It's possible to use variables in most parts of the request definition, for values that could change depending on the context in which you're sending the request. Variables are denoted by surrounding them with curly braces. The ways that variables can be bound will be explained later, but first, this example shows all the places that variables can be used:

```toml
[request]
url = "https://httpbin.org/post?something={var1}" # In the URL
method = "POST"

[query]
params = [
  { name = "{the_name}", value = "{the_value}" } # In query parameters
]

[headers]
headers = [
  { name = "Authorization", value = "Bearer {token}" } # In headers
]

# In request bodies
[body]
type = "json"
content = '''
{
  "some_key": "{something}",
  "{something_else}": 123
}'''

```

One way to bind variables is to use the `-b` or `--binding` command-line argument:

```
$ rhc -b token=xyz -b something=12345 -f definition.toml
```

The other ways to bind variables involve environments and rhc's interactive mode, which will be explained next.

### Environments

Environments are predefined groups of variables that are stored in a TOML file. They have this format:
```toml
name = "Staging"
variables = [
  { name = "token", value = "xyz" },
  { name = "something", value = "12345" }
]
```

You can specify an environment file to use with the `-e` or `--environment` argument:

```
$ rhc -e staging.toml -f definition.toml
```

By doing so, all the variables defined in the environment file will be automatically bound, and specifying them via the command line is not necessary (although you can still do so, and bindings specified via the command line will take higher precedence).

The names and values of variables defined in an environment file must be TOML strings. It's still possible to use variables as, for example, JSON numbers and booleans:

```toml
# In the request definition file:
[body]
type = "json"
content = '''
{
  "a_number": {var1},
  "a_bool": {var2}
}
'''

# In the environment file:
variables = [
  { name = "var1", value = "123" },
  { name = "var2", value = "true" }
]
```
