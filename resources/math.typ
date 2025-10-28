#set page(footer: [
  #align(right)[
    #link("https://github.com/eliavaux/elisa")[$infinity$ Eliavaux] 
  ]
])

Let $S = {s_1, ..., s_n}$ be a set of $n$ standards, with concentrations $X = {x_1, ..., x_n}$ and measurements $Y = {y_1, ..., y_n}$.

The Four parameter logistic curve we will try to fit is defined as
$
f(x) = d + (a - d)/(1 + (x/c)^b)"."
$
Each of the parameters serve a purpose:
- $a$ is the lower asymptotic bound of $f$
- $d$ is the upper asymptotic bound of $f$
- $c$ determines the point of inflection of the curve
- $d$ determines the slope at the point of inflection

We will try to find a local minimum for $a, b, c$ and $d$ using gradient descent. 

When plotted on a graph with a logarithmic scaling x-axis, it has a sigmoidal shape, so we will substitute $x$ with $hat(x) = ln x <=> x = e^hat(x)$. 
Moreover we want $c$ to scale proportionally with the point of inflection, so we will substitute it with $hat(c) = ln c <=> c = e^hat(c)$. This gives us

$
f(x) = d + (a - d)/(1 + (e^hat(x) / e^hat(c))^b) = d + (a - d)/(1 + e^(b (hat(x) - hat(c))))
$

Our cost function shall be the sum of squares.

$
C(Y, X, a, b, c, d) = 1/n sum_(i=1)^n (y_i - f(x_i))^2
$

First we compute mutual parts of the derivatives

$
C(v) = 1/n sum_(i=1)^n v^2, space.quad
v(y, u) = y - u, space.quad
u(hat(x), a, b, hat(c), d) = d + (a - d)/(1 + e^(b (hat(x) - hat(c)))) \

(partial C(v)) / (partial v)
= partial/(partial v) 1/n sum_(i=1)^n v^2
= 1/n sum_(i=1)^n partial/(partial v) v^2
= 1/n sum_(i=1)^n 2 v
= 2/n sum_(i=1)^n v \

(partial v)/(partial u) = partial/(partial u) (y - u) = -1
$

We take the partial derivative of $u$ with respect to $a, b, hat(c)$ and $d$.
$
(partial u) / (partial a) &= 1 /(1 + e^(b (hat(x) - hat(c)))) \
(partial u) / (partial b) &= -((a - d) (hat(x) - hat(c)) e^(b(hat(x) - hat(c)))) / (1 + e^(b (hat(x) - hat(c))))^2  \
(partial u) / (partial hat(c)) &= (b (a - d) e^(b (hat(x) - hat(c))))/ (1 + e^(b (hat(x) - hat(c))))^2 \
(partial u) / (partial d) &= 1 / (1 + e^(b (hat(x) - hat(c))))
$


#pagebreak()

Now, putting it all together

$
(partial C(v)) / (partial a) = (partial C(v)) / (partial u) (partial u) / (partial a)
&= - 2 / n sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) 1 /(1 + e^(b (hat(x)_i - hat(c)))) \


(partial C(v)) / (partial b) = (partial C(v)) / (partial u) (partial u) / (partial b)
&= - 2 / n sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) dot (-((a - d) (hat(x)_i - hat(c)) e^(b(hat(x)_i - hat(c)))) / (1 + e^(b (hat(x)_i - hat(c))))^2) \
&= 2 / n (a - d) sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) ((hat(x)_i - hat(c)) e^(b(hat(x)_i - hat(c)))) / (1 + e^(b (hat(x)_i - hat(c))))^2 \

(partial C(v)) / (partial hat(c)) = (partial C(v)) / (partial u) (partial u) / (partial hat(c))
&= - 2 / n sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) (b (a - d) e^(b (hat(x)_i - hat(c))))/ (1 + e^(b (hat(x)_i - hat(c))))^2 \
&= - 2 / n b (a - d) sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) (e^(b (hat(x)_i - hat(c))))/ (1 + e^(b (hat(x)_i - hat(c))))^2 \

(partial C(v)) / (partial d) = (partial C(v)) / (partial u) (partial u) / (partial d)
&= - 2 / n sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) 1 / (1 + e^(b (hat(x)_i - hat(c)))) "."
$

Note that

$
& sum_(i=1)^n (y_i - d - (a - d)/(1 + e^(b(hat(x)_i - hat(c))))) dot g(x_i) \
=& sum_(i=1)^n y_i dot g(x_i) - d sum_(i=1)^n g(x_i) - (a - d) sum_(i=1)^n g(x_i) / (1 + e^(b (hat(x)_i - hat(c)))) \
$
so the function can be split up further, reducing total operations per iteration
