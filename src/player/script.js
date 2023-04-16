const PUZZLE_WIDTH = 40

class CellValue {
    constructor(id, correct) {
        this.id = id
        this.correct = correct
        this.mutable = correct.match(/[A-Z]/i)
        if (!this.mutable) {
            this.guess = this.correct
        }
    }
}

class Cell {
    constructor(value) {
        this.nodeCell = document.createElement("div")
        this.nodeText = document.createTextNode("")
        this.nodeCell.appendChild(this.nodeText)
        this.nodeCursor = document.createElement("div")
        this.nodeCell.appendChild(this.nodeCursor)
        this.value = value
    }
    render(selected_grid, selected_value) {
        this.nodeCell.className = "entry-cell "

        if (this.value.guess == " ") {
            this.nodeText.nodeValue = ""
            this.nodeCell.className += "content-space "
        } else if (this.value.mutable) {
            this.nodeText.nodeValue = this.value.guess
            this.nodeCell.className += "content-guess "
        } else {
            this.nodeText.nodeValue = this.value.correct
            this.nodeCell.className += "content-correct "
        }

        this.nodeCursor.className = "cursor "
        if (selected_grid) {
            if (selected_value == this.value) {
                this.nodeCursor.className += "cursor-both "
            } else {
                this.nodeCursor.className += "cursor-grid "
            }
        } else {
            if (selected_value == this.value) {
                this.nodeCursor.className += "cursor-cell "
            } else {
                this.nodeCursor.className += "cursor-none "
            }
        }
        // this.nodeCell.className = clazz
    }
}

class Grid {
    constructor() {
        this.nodeGridHolder = document.createElement("div")
        this.nodeGridHolder.className = "entry-grid-holder"
        this.nodeGrid = document.createElement("div")
        this.nodeGrid.className = "entry-grid"
        this.nodeGridHolder.appendChild(this.nodeGrid)
        this.cells = []
    }
    addCell(cell) {
        this.nodeGrid.appendChild(cell.nodeCell)
        this.cells.push(cell)
    }
    render(selected_grid, selected_value) {
        for (const cell of this.cells) {
            cell.render(selected_grid == this, selected_value)
        }
    }
    cell_for_value(value) {
        return this.cells.find(cell => cell.value == value)
    }
    delta_cell(cell, delta) {
        var index = this.cells.findIndex(c2 => cell == c2)
        if (delta == 1 || delta == -1) {
            index = (index + delta + this.cells.length * 100) % this.cells.length
        } else if (delta == PUZZLE_WIDTH) {
            index = index + PUZZLE_WIDTH
            if (index > this.cells.length) {
                index = index % PUZZLE_WIDTH
            }
        } else if (delta == -PUZZLE_WIDTH) {
            index = index - PUZZLE_WIDTH
            if (index < 0) {
                while (index + PUZZLE_WIDTH < this.cells.length) {
                    index += PUZZLE_WIDTH
                }
            }
        }
        var result = this.cells[index]
        console.assert(result)
        return result
    }
    delta_value(value, delta) {
        return this.delta_cell(this.cell_for_value(value), delta).value
    }
}

class Puzzle {
    constructor(puzzle) {
        this.div = document.createElement("div")
        this.puzzle = puzzle
        this.grids = []
        this.quote = new Grid()
        this.quote.nodeGrid.className += " entry-grid-quote"
        this.grids.push(this.quote)
        for (var i = 0; i < puzzle.quote_letters.length; i++) {
            var cell = new Cell(new CellValue(i, puzzle.quote_letters[i]))
            this.quote.addCell(cell, i)
        }
        this.div.appendChild(this.quote.nodeGridHolder)
        this.div.appendChild(document.createElement("br"))
        this.source = new Grid()
        this.source.nodeGrid.className += " entry-grid-source"
        this.grids.push(this.source)
        this.div.appendChild(this.source.nodeGridHolder)
        this.clues = []
        for (const [index, clue] of puzzle.clues.entries()) {
            var cell = new Cell(this.quote.cells[clue.indices[0]].value)
            this.source.addCell(cell)
            var p = document.createElement("p")
            this.div.appendChild(p)
            var grid = new Grid()
            grid.nodeGrid.className += " entry-grid-answer"
            grid.clue = clue
            p.appendChild(document.createTextNode(clue.clue))
            p.appendChild(document.createElement("br"))
            p.appendChild(grid.nodeGridHolder)
            for (var i = 0; i < clue.answer_letters.length; i++) {
                var cell = new Cell(this.quote.cells[clue.indices[i]].value)
                grid.addCell(cell)
            }
            this.clues[index] = grid
            this.grids.push(grid)
        }
        this.cursor_grid = this.quote
        this.cursor_value = this.quote.cells[0].value
        this.render()
    }
    render() {
        this.quote.render(this.cursor_grid, this.cursor_value)
        this.source.render(this.cursor_grid, this.cursor_value)
        for (var clue of this.clues) {
            clue.render(this.cursor_grid, this.cursor_value)
        }
    }
    delta_cursor(delta) {
        this.cursor_value = this.cursor_grid.delta_value(this.cursor_value, delta)
    }
    onKeydown(event) {
        if (event.key.match(/^[a-zA-Z0-9]$/)) {
            event.preventDefault()
            if (this.cursor_value.mutable) {
                this.cursor_value.guess = event.key.toUpperCase()
            }
            this.delta_cursor(1)
            this.render()
        } else if (event.code == "ArrowLeft") {
            event.preventDefault()
            this.delta_cursor(-1)
            this.render()
        } else if (event.code == "ArrowRight") {
            event.preventDefault()
            this.delta_cursor(1)
            this.render()
        } else if (event.code == "ArrowUp") {
            event.preventDefault()
            if (this.cursor_grid == this.quote) {
                this.delta_cursor(-PUZZLE_WIDTH)
                this.render()
            }
        } else if (event.code == "ArrowDown") {
            event.preventDefault()
            if (this.cursor_grid == this.quote) {
                this.delta_cursor(PUZZLE_WIDTH)
                this.render()
            }
        } else if (event.code == "Tab") {
            event.preventDefault()
            var delta = 1
            if (event.shiftKey) {
                delta = -1
            }
            this.cursor_grid = this.grids[(this.grids.indexOf(this.cursor_grid) + delta + this.grids.length) % this.grids.length]
            this.cursor_value = this.cursor_grid.cells[0].value
            this.render()
        } else if (event.code == "Backspace") {
            event.preventDefault()
            this.delta_cursor(-1)
            if (this.cursor_value.mutable) {
                this.cursor_value.guess = ""
            }
            this.render()
        }
    }
}


async function main() {
    var params = new URLSearchParams(window.location.search)
    var url = params.get("puzzle");
    var data = await fetch(params.get("puzzle"));
    var puzzle = await data.json()
    console.log(puzzle.quote_letters)
    puzzle = new Puzzle(puzzle)
    document.body.appendChild(puzzle.div)
    document.addEventListener('keydown', function (event) { puzzle.onKeydown(event) });
}

main()