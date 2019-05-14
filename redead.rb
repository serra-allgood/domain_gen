File.open('dead.txt', 'r') do |file|
  File.open('redead.txt', 'w') do |new_file|
    period = 0
    file.each_char do |chr|
      period += 1 if chr == '.' || !period.zero?
      if period == 3
        new_file.write(chr)
        new_file.write("\n")
        period = 0
      else
        new_file.write(chr)
      end
    end
  end
end
