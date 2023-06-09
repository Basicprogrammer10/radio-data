using Plots, SpecialFunctions, Crayons
include("random_lib.jl")

# The number of bytes to load from the server
# If missing, it will get the buffer size from the server
BUFFER_SIZE = missing

# Converts an array of bytes to an array of bits
# Not bitpacked :sob:
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

# Load the buffer
buffer = load_buffer(buffer_size)
println("[*] Buffer loaded")


# Monobit Test, used to check if the sequence is random
bits = bit_vec(buffer)
n = length(bits)

o = sum(bits)
z = n - o
println("[*] Bit Ratio: $o/$z = $(o/z)")

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
passed = p >= 0.01
style = Crayon(foreground=passed ? :green : :red)
println(style, " | Sequence is $(passed ? "random (>= 0.01)" : "not random (< 0.01))")")
print(Crayon(reset=true))

# Plot the buffer as a bar chart
display(histogram(buffer, nbins=256, title="Buffer", xlabel="Value", ylabel="Count", size=(1000, 1000)))
println("[*] Showing Plot")
