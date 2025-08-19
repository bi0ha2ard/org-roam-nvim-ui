local res = vim.system(
  { "/home/felix/git/org-roam-nvim-ui/target/debug/rpc_test" },
  {
    stdin=true,
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
    print("job exited with ", vim.inspect(completed))
  end
)

res:write("hello\n")
res:write("quit\n");
