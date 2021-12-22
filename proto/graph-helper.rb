#! /usr/bin/env ruby
# Usage: `ruby graph-helper.rb <path-for-vg> <path-for-xg> <chr_prefix>`
# JSON's input is required via STDIN.

require 'json'
require 'open3'

bin = ARGV[0]
XG = ARGV[1]
CHR_PREFIX = ARGV[2] || "chr"

bin = bin.split(' ') if bin
BIN_DOCKER = bin ? bin[0..-2] : 'echo'
BIN = bin ? bin[-1] : 'echo'

def sankey(json_data)
  json_hash = {}
  json_hash[:links] = []
  json_hash[:nodes] = []

  node_hash = {}
  path_hash = Hash.new {|k, v| k[v] = {} }
  edge_hash = Hash.new {|k, v| k[v] = [] }
  edge_list = []
  edge_ctr = 0

  if json_data['path']
  json_data['path'].each do |path|
    pathname = (path['name'].start_with?(CHR_PREFIX) ? "" : CHR_PREFIX) + path["name"]
    path['mapping'].each do |t|
      path_hash[t['position']['node_id']].store(pathname, [t['rank']])
    end
    path['mapping'].each_cons(2) do |a,b|
      if b
        edge_hash[a['position']['node_id']] << edge_ctr
        edge_hash[b['position']['node_id']] << edge_ctr
        edge_list << pathname
        edge_ctr += 1
      end
    end
    node_list = path['mapping'].map{|t| t['position']['node_id']}.join(" ")
    cmd = [BIN, "find -N <( echo", node_list, ") -P", path['name'], "-x", XG]
    if BIN_DOCKER == []
      cmd2 = ["bash", "-c \"", cmd.join(" "), "\""]
    else
      cmd2 = [BIN_DOCKER, "bash", "-c \"", cmd.join(" "), "\""]
    end
    o, _, _ = Open3.capture3(cmd2.join(" "))
    o.split("\n").map {|t| t.split("\t")}.each do |t|
      if t.length > 1
        path_hash[t[0].to_i][pathname] << t[1].to_i
      end
    end
  end
  end

  if json_data['node']
    json_data['node'].each_with_index do |t, i|
      node_hash[t['id']] = i #NodeID to UUID
      json_hash[:nodes] << {name: t['id'].to_s, length: Math.log2(t['sequence'].length+1)/10, sequence: t['sequence'].length, path: path_hash[t['id']], raw_seq: t['sequence']}
    end
  end

  if json_data['edge']
  json_data['edge'].each do |edge|
    if edge['from'] != edge['to'] # It ignores repeat currently.
      if (edge_hash[edge['from']] & edge_hash[edge['to']]).length >= 1
        (edge_hash[edge['from']] & edge_hash[edge['to']]).each do |edge_id|
          value = 4 #FIXME()This is Magic number.
          path = edge_list[edge_id]
          path_start = path_hash[edge['from']][path][1]
          json_hash[:links] << {source: node_hash[edge['from']], target: node_hash[edge['to']], value: value, path: path, coord: path_start }
        end
      else
        value = 1
        path = ""
        path_start = ""
        json_hash[:links] << {source: node_hash[edge['from']], target: node_hash[edge['to']], value: value, path: path, coord: path_start }
      end
    end
  end
  end
  json_hash
end

input = JSON.parse(STDIN.read)
puts JSON.dump(sankey(input)) if input
