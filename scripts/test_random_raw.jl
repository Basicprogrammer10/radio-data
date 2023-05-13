using Plots, SpecialFunctions
include("random_lib.jl")

BUFFER_SIZE = 10_000 # missing

function bit_vec(values::Array{UInt8})
    bits = falses(length(values) * 8)
    for i in 1:length(values)
        for j in 1:8
            bits[(i-1)*8+j] = (values[i] >> (j - 1)) & 1 == 1
        end
    end

    bits
end

# Get random buffer from API
buffer_size = if BUFFER_SIZE === missing
    get_buffer()
else
    BUFFER_SIZE
end
buffer = load_buffer(buffer_size)
println("[*] Buffer loaded")


# Monobit Test
bits = bit_vec(buffer)
n = length(bits)
s = 0
for i in eachindex(bits)
    if bits[i]
        global s += 1
    else
        global s -= 1
    end
end
so = abs(s) / sqrt(n)
p = erfc(so / sqrt(2))
println("[*] Monobit Test: $p")
if p < 0.01
    println(" | Sequence is not random")
else
    println(" | Sequence is random")
end

# Plot the buffer as a bar chart
display(histogram(buffer, nbins=256, title="Buffer", xlabel="Value", ylabel="Count", size=(1000, 1000)))
println("[*] Showing Plot")
