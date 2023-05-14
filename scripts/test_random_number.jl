using Plots, ProgressBars
include("random_lib.jl")

# The number of random values to generate
COUNT = 10_000
# The range of the random values
RANGE = 0.0:100.0

# Load COUNT floats in the specified range, adding them to the buffer
values = zeros(COUNT)
for i in ProgressBar(1:COUNT)
    values[i] = get_float(RANGE[1], RANGE[end])
end


# Plot the buffer as a bar chart
display(histogram(values, title="Random Values", xlabel="Value", ylabel="Count", size=(1000, 1000)))