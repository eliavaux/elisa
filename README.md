## Description

A tool for four parameter logistic curve fitting for assay analysis, witten in egui.
Uses gradient descent.

To build the app, simply run `cargo run --release` from the project directory.


## Thoughts

My goal was to create an open source, user-friendly application for 4PL curve fitting.

I was unable to find any existing comprehensible, open source algorithms which implement curve
fit specifically for 4PL, and so I had to write the code from scratch.
Moreover most scientific articles I could find on 4PL curve fitting were promoting their own product,
rather than explaining how it works.

The gradient descent solution I implemented seems to yield decent results.
Finding the global minimum, rather than a local one with gradient descent, would be ideal.

I plan to add support for 5PL as well.

## Resources
Since I couldn't find them anywhere else, here are the derivatives of the error function with respect
to a, b, c and d which I used for gradient descent.

![Derivatives with respect to a, b, c and d](https://github.com/eliavaux/elisa/blob/main/resources/math.jpg)
