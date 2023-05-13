using Formatting
using HTTP, JSON

HOST = "http://localhost:8080"
SLEEP_TIME = 3 # seconds

function get_buffer()
    r = HTTP.get("$HOST/status")
    body = JSON.parse(String(r.body))

    buffer_filled = body["buffer_filled"]
    percent_filled = body["percent_filled"]
    printfmtln("[*] Buffer filled: {} ({:.1}%)", buffer_filled, percent_filled * 100)

    if percent_filled < 1.0
        println(" | Buffer not filled, waiting...")
        sleep(SLEEP_TIME)
        return get_buffer()
    end

    return buffer_filled
end

function load_buffer(x::Int)
    r = HTTP.get("$HOST/raw/$x")
    if r.status != 200
        println("[-] Error loading buffer")
        exit(1)
    end

    return r.body
end

function get_float(min::Float64, max::Float64)
    r = HTTP.get("$HOST/data/number/$min/$max")
    if r.status != 200
        println("[-] Error getting float")
        exit(1)
    end

    return parse(Float64, String(r.body))
end