#!/bin/sh

if ! [ -f "./book.toml" ]; then
  echo "This script must be run from the root of the mdbook project."
  exit 1
fi

doc_dir=$(pwd)

# # cleanup on any exit (normal or error)
# trap 'rm -rf "$temp"' EXIT

cargo doc --no-deps --target-dir "$doc_dir/target-doc" --example book_doc
if [ $? -ne 0 ]; then
  echo "Failed to generate documentation for the 'book_doc' example."
  exit 1
fi

"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --headless \
    --no-pdf-header-footer \
    --print-to-pdf=./target-doc/book_doc.pdf \
    ./target-doc/doc/book_doc/struct.Book.html >/dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "Failed to generate PDF documentation for the Book type of 'book_doc' example using Chrome."
    exit 1
fi

pdftocairo -png -f 1 -l 1 ./target-doc/book_doc.pdf ./src/img/basics_book_doc.png >/dev/null
if [ $? -ne 0 ]; then
    echo "Failed to convert PDF to PNG for the Book type."
    exit 1
fi

"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --headless \
    --no-pdf-header-footer \
    --print-to-pdf=./target-doc/book_doc_builder.pdf \
    ./target-doc/doc/book_doc/struct.BookBuilder.html >/dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "Failed to generate PDF documentation for the BookBuilder type of 'book_doc' example using Chrome."
    exit 1
fi

pdftocairo -png -f 1 -l 1 ./target-doc/book_doc_builder.pdf ./src/img/basics_book_doc_builder.png >/dev/null
if [ $? -ne 0 ]; then
    echo "Failed to convert PDF to PNG for the BookBuilder type."
    exit 1
fi
