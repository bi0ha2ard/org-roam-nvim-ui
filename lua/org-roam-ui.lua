local process = nil;

local M = {}

M.start = function()
  if process then
    return
  end
  process = vim.system(
    { vim.loop.os_homedir() .. "/.cargo/bin/org-roam-nvim-ui" },
    {
      stdin = true,
      stdout = function(err, data)
        assert(not err, err)
        if data then
          vim.schedule(function()
            vim.notify("stdin " .. data)
          end)
        end
      end,
      stderr = function(err, data)
        assert(not err, err)
        if data then
          vim.schedule(function()
            vim.notify("stderr: " .. data)
          end)
        end
      end
    },
    function(completed)
      -- print("job exited with ", vim.inspect(completed))
      process = nil;
    end
  )
end

local roam = require('org-roam')

M.open_at = function(file)
  M.start()
  roam.database:find_nodes_by_file(file):next(function(nodes)
    for _, node in ipairs(nodes) do
      if process then
        process:write("select " .. node.id .. "\n")
      end
      break
    end
  end)
end

M.open = function(buf)
  local fname = vim.api.nvim_buf_get_name(buf or 0);
  if fname == "" then
    return
  end
  M.open_at(fname)
end

M.quit = function()
  if process then
    process:write("quit\n")
  end
end

return M
