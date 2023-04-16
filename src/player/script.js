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
        // this.nodeCell.addEventListener("click", function (e) {
        //     console.log(e)
        // })
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
    constructor(puzzle) {
        this.nodeGridHolder = document.createElement("div")
        this.nodeGridHolder.className = "entry-grid-holder"
        this.nodeGrid = document.createElement("div")
        this.nodeGrid.className = "entry-grid"
        this.nodeGridHolder.appendChild(this.nodeGrid)
        var grid = this
        this.nodeGrid.addEventListener("click", function (e) {

            for (const cell of grid.cells) {
                var rect = cell.nodeCell.getBoundingClientRect();
                var x = e.clientX - rect.left; //x position within the element.
                var y = e.clientY - rect.top;
                if (e.clientY > rect.top && e.clientY < rect.bottom && e.clientX > rect.left - 10 && e.clientX < rect.right - 10) {
                    puzzle.cursor_value = cell.value
                    puzzle.cursor_grid = grid
                    puzzle.render()
                    break;
                }
            }
        })
        this.cells = []
        this.puzzle = puzzle
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
    constructor(url, puzzle) {
        this.div = document.createElement("div")
        this.puzzle = puzzle
        this.grids = []
        this.quote = new Grid(this)
        this.quote.nodeGrid.className += " entry-grid-quote"
        this.grids.push(this.quote)
        for (var i = 0; i < puzzle.quote_letters.length; i++) {
            var cell = new Cell(new CellValue(i, puzzle.quote_letters[i]))
            this.quote.addCell(cell, i)
        }
        this.div.appendChild(this.quote.nodeGridHolder)
        this.div.appendChild(document.createElement("br"))
        this.source = new Grid(this)
        this.source.nodeGrid.className += " entry-grid-source"
        this.grids.push(this.source)
        this.div.appendChild(this.source.nodeGridHolder)
        this.clues = []
        this.nodeClues = document.createElement("div")
        this.nodeClues.className = "clues-holder"
        this.div.appendChild(document.createElement("br"))
        this.div.appendChild(this.nodeClues)
        for (const [index, clue] of puzzle.clues.entries()) {
            var cell = new Cell(this.quote.cells[clue.indices[0]].value)
            this.source.addCell(cell)
            var p = document.createElement("div")
            this.nodeClues.appendChild(p)
            var grid = new Grid(this)
            grid.nodeGrid.className += " entry-grid-answer"
            grid.clue = clue
            p.appendChild(document.createTextNode((index + 1) + ". " + clue.clue))
            p.appendChild(document.createElement("br"))
            p.appendChild(grid.nodeGridHolder)
            p.appendChild(document.createElement("br"))
            for (var i = 0; i < clue.answer_letters.length; i++) {
                var cell = new Cell(this.quote.cells[clue.indices[i]].value)
                grid.addCell(cell)
            }
            this.clues[index] = grid
            this.grids.push(grid)
        }
        this.cursor_grid = this.quote
        this.cursor_value = this.quote.cells[0].value
        this.url = url
        this.loadFromStorage()
        this.render()
    }
    loadFromStorage() {
        var local = JSON.parse(localStorage.getItem(this.url))
        if (local && local.guesses) {
            for (const [index, cell] of this.quote.cells.entries()) {
                if (cell && cell.value.mutable && local.guesses[index]) {
                    cell.value.guess = local.guesses[index]
                }
            }
        }
    }
    saveToStorage() {
        var guesses = []
        for (const cell of this.quote.cells) {
            guesses.push(cell.value.mutable ? cell.value.guess : null)
        }
        localStorage.setItem(this.url, JSON.stringify({ guesses: guesses }))
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
        if (event.key.match(/^[a-zA-Z0-9]$/) && !event.metaKey && !event.ctrlKey) {
            event.preventDefault()
            if (this.cursor_value.mutable) {
                this.cursor_value.guess = event.key.toUpperCase()
            }
            this.delta_cursor(1)
            this.saveToStorage()
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
        } else if (event.code == "Tab" || event.code == "Enter") {
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
            this.saveToStorage()
            this.render()
        }
    }
}

class Index {
    constructor(index) {
        this.div = document.createElement("p")
        for (const p of index.links) {
            let a = document.createElement("a")
            a.href = ".?puzzle=" + encodeURIComponent(p.url)
            let t = document.createTextNode(p.name)
            a.appendChild(t)
            this.div.appendChild(a)
            this.div.appendChild(document.createElement("br"))
        }
    }
}

async function load_puzzle(url) {
    var data = await fetch(url);
    var puzzle = await data.json()
    puzzle = new Puzzle(url, puzzle)
    document.getElementById("contents").appendChild(puzzle.div)
    document.addEventListener('keydown', function (event) { puzzle.onKeydown(event) });
}

async function load_index(url) {
    var data = await fetch(url);
    var index = await data.json()
    index = new Index(index)
    document.getElementById("contents").appendChild(index.div)
}

async function main() {
    var params = new URLSearchParams(window.location.search)
    var puzzle = params.get("puzzle");
    var index = params.get("index");
    if (puzzle) {
        await load_puzzle(puzzle)
    } else if (index) {
        await load_index(index)
    } else {
        load_index("./puzzles.json")
    }

}

window.onload = main