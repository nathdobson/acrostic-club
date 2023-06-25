CELL_SIZE = 10

class CellValue {
    constructor(id, correct) {
        this.id = id
        this.correct = correct
        this.mutable = correct.match(/[A-Z]/i)
        this.marker = ""
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
            if (this.value.marker) {
                this.nodeCell.className += "content-guess-" + this.value.marker + " "
            }
        } else {
            this.nodeText.nodeValue = this.value.correct
            this.nodeCell.className += "content-correct "
        }

        this.nodeCursor.className = "cursor "
        if (selected_grid) {
            if (selected_value == this.value) {
                this.nodeCursor.className += "cursor-both "
                if (this.value.mutable) {
                    this.nodeCell.className += "content-guess-both "
                }
            } else {
                this.nodeCursor.className += "cursor-grid "
                if (this.value.mutable) {
                    this.nodeCell.className += "content-guess-grid "
                }
            }
        } else {
            if (selected_value == this.value) {
                this.nodeCursor.className += "cursor-cell "
                if (this.value.mutable) {
                    this.nodeCell.className += "content-guess-cell "
                }
            } else {
                this.nodeCursor.className += "cursor-none "
                if (this.value.mutable) {
                    this.nodeCell.className += "content-guess-none "
                }
            }
        }
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
            for (var i = 0; i < grid.cells.length; i++) {
                var cell = grid.cells[i]
                var rect = cell.nodeCell.getBoundingClientRect();
                var x = e.clientX - rect.left; //x position within the element.
                var y = e.clientY - rect.top;
                var right = rect.right
                if (i == grid.cells.length - 1) {
                    right = Infinity
                }
                if (e.clientY >= rect.top && e.clientY < rect.bottom && e.clientX >= rect.left - CELL_SIZE && e.clientX < right - CELL_SIZE) {
                    puzzle.cursor_value = cell.value
                    puzzle.cursor_grid = grid
                    puzzle.render()
                    return;
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
        let puzzle_width = window.getComputedStyle(this.nodeGrid).gridTemplateColumns.split(" ").length
        if (delta == 1 || delta == -1) {
            index = (index + delta + this.cells.length * 100) % this.cells.length
        } else if (delta == 2) {
            index = index + puzzle_width
            if (index > this.cells.length) {
                index = index % puzzle_width
            }
        } else if (delta == -2) {
            index = index - puzzle_width
            if (index < 0) {
                while (index + puzzle_width < this.cells.length) {
                    index += puzzle_width
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
    constructor(url, puzzle, socket) {
        this.div = document.createElement("div")
        this.puzzle = puzzle
        this.socket = socket
        this.grids = []
        if (!this.socket) {
            var a = document.createElement("a")
            a.appendChild(document.createTextNode("Enable multiplayer"))
            a.setAttribute("href", window.location.href + "&room=wss://ws.acrostic.club/room/" + (Math.random() + 1).toString(36).substring(7))
            this.div.appendChild(a)
        } else {
            let share_box = document.createElement("p")
            let input = document.createElement("input")
            input.type = "text"
            input.value = window.location.href
            input.readonly = "readonly"
            share_box.appendChild(input)
            let button = document.createElement("button")
            button.appendChild(document.createTextNode("Copy multiplayer link"))
            let div = document.createElement("span")
            alert = document.createTextNode("Copied multiplayer link to clipboard.")
            div.style.display = "none"
            button.addEventListener("click", () => {
                input.select()
                input.setSelectionRange(0, 9999)
                navigator.clipboard.writeText(window.location.href)
                if (div.style.display == "none") {
                    div.style.display = ""
                    setTimeout(() => {
                        div.style.display = "none"
                    }, 3000)
                }
            })
            share_box.appendChild(button);
            div.appendChild(alert)
            share_box.appendChild(div)
            this.div.appendChild(share_box)
        }
        this.pencilp = document.createElement("p")
        this.pencil = document.createElement("input")
        this.pencil.type = "checkbox"
        this.pencil.id = "pencil"
        this.pencill = document.createElement("label")
        this.pencill.htmlFor = "pencil"
        this.pencill.appendChild(document.createTextNode("Pencil (key: `)"))
        this.pencilp.appendChild(this.pencil)
        this.pencilp.appendChild(this.pencill)
        this.div.appendChild(this.pencilp)

        this.solution = document.createElement("p")
        this.solution.className = "solution"
        this.solution.appendChild(document.createTextNode(puzzle.quote))
        this.solution.appendChild(document.createElement("br"))
        this.solution.appendChild(document.createTextNode(puzzle.source))
        this.solution.style.display = "none"
        this.div.appendChild(this.solution)

        this.quote = new Grid(this)
        this.quote.nodeGrid.className += " entry-grid-quote"
        this.quote.nodeGridHolder.className = "entry-grid-holder-quote"
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
    }
    loadFromStorage() {
        var local = JSON.parse(localStorage.getItem(this.url))
        var upload = {}
        if (local && local.guesses) {
            for (const [index, cell] of this.quote.cells.entries()) {
                if (cell && cell.value.mutable && local.guesses[index]) {
                    cell.value.guess = local.guesses[index]
                    upload[index] = { time: 1, breaker: 1, guess: local.guesses[index] }
                }
            }
        }
        if (local && local.values) {
            for (const [index, cell] of this.quote.cells.entries()) {
                if (cell && cell.value.mutable && local.values[index]) {
                    cell.value.guess = local.values[index].guess
                    cell.value.marker = local.values[index].marker
                    upload[index] = {
                        time: 1, breaker: 1, guess: local.values[index].guess, marker: local.values[index].marker
                    }
                }
            }
        }
        if (this.socket) {
            upload = JSON.stringify(upload)
            this.socket.send(upload)
        }
    }
    saveToStorage() {
        var values = []
        for (const cell of this.quote.cells) {
            values.push(cell.value.mutable ? {
                guess: cell.value.guess, marker: cell.value.marker
            } : null)
        }
        localStorage.setItem(this.url, JSON.stringify({ values: values }))
    }
    render() {
        this.quote.render(this.cursor_grid, this.cursor_value)
        this.source.render(this.cursor_grid, this.cursor_value)
        for (var clue of this.clues) {
            clue.render(this.cursor_grid, this.cursor_value)
        }
        var incorrect = 0
        for (var cell of this.quote.cells) {
            if (cell.value.guess != cell.value.correct) {
                incorrect += 1
            }
        }
        this.solution.style.display = incorrect == 0 ? "block" : "none"
    }
    delta_cursor(delta) {
        this.cursor_value = this.cursor_grid.delta_value(this.cursor_value, delta)
    }
    set_guess(guess, marker) {
        var time = Date.now();
        if (this.cursor_value.time >= time) {
            time = this.cursor_value.time + 1
        }
        var breaker = Math.round(Math.random() * 1000000000);
        this.cursor_value.guess = guess
        this.cursor_value.marker = marker
        this.cursor_value.time = time
        this.cursor_value.breaker = breaker
        if (this.socket) {
            let message = {}
            message[this.cursor_value.id] = { time, breaker, guess: guess, marker: marker }
            this.socket.send(JSON.stringify(message))
        }
    }
    set_guess_at(position, guess, marker, time, breaker) {
        var value = this.quote.cells[position].value
        if (time < value.time) {
            return;
        }
        if (time == value.time && breaker <= value.breaker) {
            return;
        }
        value.guess = guess
        value.marker = marker
        value.time = time
        value.breaker = breaker
        this.render()
    }
    onKeydown(event) {
        if (event.metaKey || event.ctrlKey) {
            return
        }
        if (event.key.match(/^[a-zA-Z0-9]$/)) {
            event.preventDefault()
            if (this.cursor_value.mutable) {
                this.set_guess(event.key.toUpperCase(), this.pencil.checked ? "pencil" : "pen")
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
                this.delta_cursor(-2)
                this.render()
            }
        } else if (event.code == "ArrowDown") {
            event.preventDefault()
            if (this.cursor_grid == this.quote) {
                this.delta_cursor(2)
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
                this.set_guess("", "")
            }
            this.saveToStorage()
            this.render()
        } else if (event.code == "Space") {
            event.preventDefault()
            if (this.cursor_value.mutable) {
                this.set_guess("", "")
            }
            this.delta_cursor(1)
            this.saveToStorage()
            this.render()
        } else if (event.key == "`") {
            event.preventDefault()
            this.pencil.checked = !this.pencil.checked
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

async function load_puzzle(url, room) {
    var data = await fetch(url);
    var socket
    if (room) {
        socket = new WebSocket(room);
    }
    var puzzle = await data.json()
    puzzle = new Puzzle(url, puzzle, socket)
    document.getElementById("contents").appendChild(puzzle.div)
    document.addEventListener('keydown', function (event) { puzzle.onKeydown(event) });
    if (room) {
        socket.addEventListener("open", (event) => {
            console.log("connected websocket", event)
        });
        first_message = true
        socket.addEventListener("message", (event) => {
            let data = JSON.parse(event.data)
            if (first_message) {
                if (Object.keys(data).length == 0) {
                    puzzle.loadFromStorage()
                }
            }
            first_message = false
            for (var x in data) {
                puzzle.set_guess_at(x, data[x].guess, data[x].marker, data[x].time, data[x].breaker)
            }
            puzzle.saveToStorage();
            puzzle.render()
        });

        socket.addEventListener("error", (event) => {
            console.log("Error ", event);
        });
    } else {
        puzzle.loadFromStorage()
    }
    puzzle.render()

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
    var room = params.get("room");

    if (puzzle) {
        await load_puzzle(puzzle, room)
    } else if (index) {
        await load_index(index)
    } else {
        load_index("./puzzles.json")
    }
}

window.onload = main