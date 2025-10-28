## Description

A tool for four parameter logistic curve fitting for assay analysis, witten in egui.
Uses gradient descent.

To build the app, simply run `cargo build --release` from the project directory.


## Thoughts

My goal was to create an open source, user-friendly application for 4PL curve fitting.

I was unable to find any existing comprehensible, open source algorithms which implement curve fit
specifically for 4PL, and so I had to write the code from scratch.
Moreover I found that most scientific articles on 4PL curve fitting were promoting their own product,
rather than explaining how it works.

The gradient descent solution I implemented seems to yield decent results.
Finding the global minimum, rather than a local one with gradient descent, would be ideal.

I plan to add support for 5PL as well.

## Resources

### Screenshots

<img src="https://github.com/eliavaux/elisa/blob/main/resources/Screenshot%20Microplate.png" width="50%">
<img src="https://github.com/eliavaux/elisa/blob/main/resources/Screenshot%20Plot.png" width="50%">

### Math

Since I couldn't find much on the topic anywhere else, I wrote a short explanation. The pdf can be found in `resources/math.pdf`

<img src="https://github.com/eliavaux/elisa/blob/main/resources/math1.svg" width="50%">
<img src="https://github.com/eliavaux/elisa/blob/main/resources/math2.svg" width="50%">
