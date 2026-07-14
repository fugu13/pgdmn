# Examples and code

How a visitor gets from reading about pgdmn to running it: finding the scenario that matches their problem, downloading its files, reading the SQL, and going deeper if they want to.

## Finding the right example

The Examples page opens with a short list of the scenarios it covers — each a name and a one-line description of what it decides. Choosing one jumps to that scenario further down the same page.

The list exists so the first question a visitor has ("is my problem in here?") is answered without scrolling through four worked examples to find out.

## An example

Each scenario follows the same shape, in the order a visitor needs it.

First its name and a sentence saying what it decides. Then a row of the files it uses: the decision model and the dataset, each shown with an icon indicating which is which, and named by filename so it is obvious what lands in the downloads folder. Choosing one saves it rather than opening it — a visitor who clicks a model does not get a screenful of raw XML they have to navigate back out of.

Then the SQL. It begins at the query, not at setup: the point of the page is what a decision looks like when you ask for one, not how to create a table. Where the result matters, it is shown as a comment beneath the statement, laid out the way the database actually prints it, so a visitor can compare what they got against what they should have got without leaving the page.

SQL is syntax-highlighted. Colour is decoration only — every token remains legible in the body colour if colour is unavailable or unseen.

Finally, a link out to the complete walkthrough for that scenario, named for the scenario it explains.

## Reading code comfortably

A code block wider than the screen scrolls sideways within itself; the page as a whole never scrolls sideways. Because those blocks scroll, they can be focused and scrolled with the keyboard alone, and show a clear focus indicator when reached that way.

Each block is announced with a description of what it does, rather than every block on the page being announced identically as "SQL example".

## Going deeper

Each scenario has a walkthrough — a blog post that shows the decision table itself, every row of the sample data with the decision reached for it, and an explanation of why the answers are what they are. This is where the rules and results tables live; the Examples page deliberately does not carry them, so that it stays scannable.

A walkthrough links back to the example it explains, so a reader can move between "show me" and "explain it" in either direction. It also links to the install instructions, for a reader who arrived at the explanation before they had the extension.

## Knowing the examples are real

The models and the results shown are covered by the project's automated tests. A visitor is being asked to paste SQL into their own database, and the results printed in the comments are the results the engine actually produces — not an author's recollection of them.
