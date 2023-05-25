# im only making this as procrastination

using Plots

POINTS = 1000
FREQ = 550
RANGE = 2 * pi * 10
SHIFT = 1

IN_POINT = POINTS / (RANGE / (2 * pi))
OUT_POINT = POINTS - IN_POINT

smooth = zeros(POINTS)
normal = zeros(POINTS)
x = range(0, RANGE + SHIFT, length=POINTS)

for i in eachindex(x)
    smooth[i] = sin(x[i])
    if i < IN_POINT
        smooth[i] *= i / IN_POINT
    end
    if i > OUT_POINT
        smooth[i] *= (POINTS - i) / IN_POINT
    end

    normal[i] = sin(x[i])
end

normal[1] = 0
normal[end] = 0

normal_plot = plot(x, normal, title="Normal Tone", xlabel="Time", ylabel="Amplitude")
smooth_plot = plot(x, smooth, title="Smoothed Tone", xlabel="Time", ylabel="Amplitude")
plot(normal_plot, smooth_plot, layout=(2, 1), legend=false, size=(1000, 1000))