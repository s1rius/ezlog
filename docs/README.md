# Contribute to documentation

this documentation build by [mdbook](https://rust-lang.github.io/mdBook/)

## Build and open

```
mdbook build
```

or see real-time updates 

```
mdbook serve --open
```

## i18n

we use [mdbook-i18n-helpers](https://github.com/google/mdbook-i18n-helpers) to support multiple language.

### Genrate the PO Template

```
MDBOOK_OUTPUT='{"xgettext": {"pot-file": "messages.pot"}}' \
mdbook build -d po
```
### Initialize a New Translation

```
msginit -i po/messages.pot -l xx -o po/xx.po
```

### Updating an Existing Translation
```
msgmerge --update po/xx.po po/messages.pot
```

### serve a Translate Book
```
MDBOOK_BOOK__LANGUAGE=zh mdbook serve -d book/zh --open
```
