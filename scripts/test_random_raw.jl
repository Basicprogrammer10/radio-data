using Plots
include("random_lib.jl")

buffer_size = get_buffer()
buffer = load_buffer(buffer_size)
println("[*] Buffer loaded")

# Plot the buffer as a bar chart
display(histogram(buffer, nbins=256, title="Buffer", xlabel="Value", ylabel="Count", size=(1000, 1000)))
println("[*] Showing Plot")