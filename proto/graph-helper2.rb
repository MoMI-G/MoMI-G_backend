#! /usr/bin/env ruby
# Usage: `ruby graph-helper2.rb <path-for-vg> <path-for-xg>`
# JSON's input is required via STDIN.

require 'json'
require 'open3'

bin = ARGV[0]
XG = ARGV[1]
path = ARGV[2]
chunk_prefix = ARGV[3] if ARGV[3]

bin = bin.split(" ") if bin
BIN_DOCKER = bin ? bin[0..-2] : "echo"
BIN = bin ? bin[-1] : "echo"

def add_coordinate(json_data)
  if json_data['path']
    json_data['path'].each_with_index do |path, index|
      node_list = path['mapping'].map{|t| t['position']['node_id']}.join(" ")
      if BIN_DOCKER == []
        cmd = [BIN, "find -N <( echo", node_list, ") -P", path['name'], "-x", XG]
        cmd2 = ["bash", "-c \"", cmd.join(" "), "\""]
      else
        cmd = [BIN, "find -N <( echo", node_list, ") -P", path['name'], "-x", XG]
        cmd2 = [BIN_DOCKER, "bash", "-c \"", cmd.join(" "), "\""]
      end
      o,_,_ = Open3.capture3(cmd2.join(" "))
      pathname = path['name'].start_with?("chr")
      if o.split("\n").map{|t| t.split("\t")}[0] && o.split("\n").map{|t| t.split("\t")}[0][1] && pathname
        json_data['path'][index]['indexOfFirstBase'] = o.split("\n").map{|t| t.split("\t")}[0][1].to_i
        o.split("\n").map{|t| t.split("\t")}.each do |t|
          if t.length > 1
            node_index = json_data['path'][index]["mapping"].find_index{ |i|  i["position"]["node_id"] == t[0].to_i }
            json_data['path'][index]['mapping'][node_index]["position"]["coordinate"] = t[1].to_i if node_index
          end
        end
      end
    end
  end
  json_data
end

def add_gam(input, chunk_prefix)
  chunk_filename = Dir.glob(chunk_prefix + "*.gam")[0]
  cmd = [BIN, "view -j -a", chunk_filename]
  if BIN_DOCKER == []
    cmd2 = ["bash", "-c \"", cmd.join(" "), "\""]
  else
    cmd2 = [BIN_DOCKER, "bash", "-c \"", cmd.join(" "), "\""]
  end
  o2,_,_ = Open3.capture3(cmd2.join(" "))
  input["gam"] = o2.split("\n").map{|i| JSON.load(i)}
  input
end

input = JSON.parse(STDIN.read)
result = input ? add_coordinate(input) : {}
result = chunk_prefix ? add_gam(result, chunk_prefix) : result
puts JSON.dump(result) #if result != {}
